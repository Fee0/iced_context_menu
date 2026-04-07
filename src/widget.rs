use std::rc::Rc;
use std::time::Instant;

use iced::advanced::layout::{self, Layout};
use iced::advanced::overlay;
use iced::advanced::renderer;
use iced::advanced::widget::tree;
use iced::advanced::widget::{Operation, Tree, Widget};
use iced::advanced::{Clipboard, Overlay as OverlayTrait, Shell};
use iced::keyboard;
use iced::mouse;
use iced::{Element, Event, Length, Point, Rectangle, Size, Theme, Vector};

use crate::context_menu::{
    clamp_panel_anchor, context_menu_overlay_panels, estimate_panel_height, ContextMenuOpen, MenuItem,
};
use crate::menu::{MenuItemId, MenuNode, MenuSpec};
use crate::style::ContextMenuStyle;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubmenuOpenMode {
    Hover,
    Click,
    HoverAndClick,
}

/// `(panel_count, open_path)` - when this changes, overlay stack depth changes; need `Tree::new`.
type OverlayStructureKey = (usize, Vec<usize>);

#[derive(Debug)]
struct InternalState {
    open: Option<ContextMenuOpen>,
    cursor: Point,
    viewport: Size,
    overlay_tree: Tree,
    /// Last reconciled overlay structure; `None` when menu is closed or before first overlay build.
    last_overlay_structure: Option<OverlayStructureKey>,
    open_path: Vec<usize>,
    hover_path: Vec<usize>,
    hover_started_at: Option<Instant>,
    focused_indices: Vec<usize>,
    panel_rects: Vec<Rectangle>,
}

impl Default for InternalState {
    fn default() -> Self {
        Self {
            open: None,
            cursor: Point::ORIGIN,
            viewport: Size::new(4096.0, 4096.0),
            overlay_tree: Tree::empty(),
            last_overlay_structure: None,
            open_path: Vec::new(),
            hover_path: Vec::new(),
            hover_started_at: None,
            focused_indices: Vec::new(),
            panel_rects: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
enum OverlayEvent {
    Close,
    Inert,
    Select(MenuItemId),
    OpenPath(Vec<usize>),
}

#[derive(Debug, Clone)]
enum RowKind {
    Action(MenuItemId),
    Disabled,
    Separator,
    Submenu { child_index: usize },
}

#[derive(Debug, Clone)]
struct RowModel {
    kind: RowKind,
}

#[derive(Debug, Clone)]
struct PanelModel {
    path: Vec<usize>,
    rows: Vec<RowModel>,
}

pub struct ContextMenu<'a, Message: Clone> {
    content: Element<'a, Message>,
    items: MenuSpec,
    on_open: Option<Message>,
    on_close: Option<Message>,
    on_select: Option<Rc<dyn Fn(MenuItemId) -> Message + 'a>>,
    style: ContextMenuStyle,
    close_on_select: bool,
    submenu_open_mode: SubmenuOpenMode,
    submenu_hover_delay_ms: u16,
}

impl<'a, Message: Clone + 'a> ContextMenu<'a, Message> {
    pub fn new(content: impl Into<Element<'a, Message>>) -> Self {
        Self {
            content: content.into(),
            items: MenuSpec::new(),
            on_open: None,
            on_close: None,
            on_select: None,
            style: ContextMenuStyle::default(),
            close_on_select: true,
            submenu_open_mode: SubmenuOpenMode::Hover,
            submenu_hover_delay_ms: 150,
        }
    }

    pub fn items(mut self, items: MenuSpec) -> Self {
        self.items = items;
        self
    }

    pub fn on_open(mut self, message: Message) -> Self {
        self.on_open = Some(message);
        self
    }

    pub fn on_close(mut self, message: Message) -> Self {
        self.on_close = Some(message);
        self
    }

    pub fn on_select(mut self, map: impl Fn(MenuItemId) -> Message + 'a) -> Self {
        self.on_select = Some(Rc::new(map));
        self
    }

    pub fn style(mut self, style: ContextMenuStyle) -> Self {
        self.style = style;
        self
    }

    pub fn close_on_select(mut self, close_on_select: bool) -> Self {
        self.close_on_select = close_on_select;
        self
    }

    pub fn submenu_open_mode(mut self, mode: SubmenuOpenMode) -> Self {
        self.submenu_open_mode = mode;
        self
    }

    pub fn submenu_hover_delay_ms(mut self, delay_ms: u16) -> Self {
        self.submenu_hover_delay_ms = delay_ms.max(1);
        self
    }

    fn close_menu(&self, state: &mut InternalState, shell: &mut Shell<'_, Message>) {
        let was_open = state.open.is_some();
        state.open = None;
        state.last_overlay_structure = None;
        state.open_path.clear();
        state.hover_path.clear();
        state.hover_started_at = None;
        state.focused_indices.clear();
        state.panel_rects.clear();
        if was_open {
            if let Some(message) = &self.on_close {
                shell.publish(message.clone());
            }
        }
    }

    fn select_item(&self, id: MenuItemId, state: &mut InternalState, shell: &mut Shell<'_, Message>) {
        if let Some(map) = &self.on_select {
            shell.publish(map(id));
        }
        if self.close_on_select {
            self.close_menu(state, shell);
        }
    }
}

impl<'a, Message: Clone + 'a> From<ContextMenu<'a, Message>> for Element<'a, Message> {
    fn from(value: ContextMenu<'a, Message>) -> Self {
        Element::new(value)
    }
}

impl<Message: Clone> Widget<Message, Theme, iced::Renderer> for ContextMenu<'_, Message> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<InternalState>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(InternalState::default())
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&[self.content.as_widget()]);
    }

    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn size_hint(&self) -> Size<Length> {
        self.content.as_widget().size_hint()
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.content
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits)
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut iced::Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        self.content.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            layout,
            cursor,
            viewport,
        );
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn Operation,
    ) {
        self.content.as_widget_mut().operate(
            &mut tree.children[0],
            layout,
            renderer,
            operation,
        );
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &iced::Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        self.content.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );

        let state = tree.state.downcast_mut::<InternalState>();

        if let Some(p) = cursor.position() {
            state.cursor = p;
        }

        if let Event::Window(iced::window::Event::Resized(size)) = event {
            state.viewport = *size;
            state.last_overlay_structure = None;
        }

        // When the menu is open, pointer-driven hover/submenu logic runs in `OverlayBridge::update`.
        // Do not skip keyboard handling for an open menu just because the content child captured an event.
        if shell.is_event_captured() && state.open.is_none() {
            return;
        }

        if let Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right)) = event
            && cursor.is_over(layout.bounds())
        {
            state.open = Some(ContextMenuOpen { at: state.cursor });
            state.last_overlay_structure = None;
            state.open_path.clear();
            state.hover_path.clear();
            state.hover_started_at = None;
            state.focused_indices.clear();
            if let Some(message) = &self.on_open {
                shell.publish(message.clone());
            }
            shell.capture_event();
            return;
        }

        if state.open.is_none() {
            return;
        }

        if let Event::Keyboard(keyboard::Event::KeyPressed { key, .. }) = event {
            let runtime = build_runtime(
                self.items.nodes(),
                &state.open_path,
                state.open.expect("checked").at,
                state.viewport,
                &self.style,
            );

            ensure_focus_len(&runtime.panels, &mut state.focused_indices);
            let active_panel = runtime.panels.len().saturating_sub(1);

            match key {
                keyboard::Key::Named(keyboard::key::Named::Escape) => {
                    self.close_menu(state, shell);
                    shell.invalidate_layout();
                    shell.capture_event();
                }
                keyboard::Key::Named(keyboard::key::Named::ArrowDown) => {
                    move_focus(&runtime.panels[active_panel], &mut state.focused_indices[active_panel], 1);
                    shell.capture_event();
                }
                keyboard::Key::Named(keyboard::key::Named::ArrowUp) => {
                    move_focus(&runtime.panels[active_panel], &mut state.focused_indices[active_panel], -1);
                    shell.capture_event();
                }
                keyboard::Key::Named(keyboard::key::Named::ArrowRight) => {
                    if let Some(row) = runtime.panels[active_panel].rows.get(state.focused_indices[active_panel])
                        && let RowKind::Submenu { child_index } = row.kind
                    {
                        state.open_path.push(child_index);
                        shell.invalidate_layout();
                        shell.capture_event();
                    }
                }
                keyboard::Key::Named(keyboard::key::Named::ArrowLeft) => {
                    if !state.open_path.is_empty() {
                        state.open_path.pop();
                        shell.invalidate_layout();
                        shell.capture_event();
                    }
                }
                keyboard::Key::Named(keyboard::key::Named::Enter) => {
                    if let Some(row) = runtime.panels[active_panel].rows.get(state.focused_indices[active_panel]) {
                        match row.kind {
                            RowKind::Action(id) => self.select_item(id, state, shell),
                            RowKind::Submenu { child_index } => {
                                state.open_path.push(child_index);
                                shell.invalidate_layout();
                            }
                            RowKind::Disabled | RowKind::Separator => {}
                        }
                        shell.capture_event();
                    }
                }
                _ => {}
            }
        }
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        self.content.as_widget().mouse_interaction(
            &tree.children[0],
            layout,
            cursor,
            viewport,
            renderer,
        )
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &iced::Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, iced::Renderer>> {
        let state = tree.state.downcast_mut::<InternalState>();
        let content_overlay = self.content.as_widget_mut().overlay(
            &mut tree.children[0],
            layout,
            renderer,
            viewport,
            translation,
        );

        if state.open.is_none() {
            return content_overlay;
        }

        let runtime = build_runtime(
            self.items.nodes(),
            &state.open_path,
            state.open.expect("checked").at,
            state.viewport,
            &self.style,
        );
        state.panel_rects = runtime.rects.clone();

        let panels = runtime
            .panels
            .iter()
            .zip(runtime.rects.iter())
            .map(|(panel, rect)| {
                let items = panel
                    .rows
                    .iter()
                    .map(|row| match row.kind {
                        RowKind::Action(id) => MenuItem::Action {
                            label: label_for_row(self.items.nodes(), &panel.path, row).unwrap_or_else(|| "Action".to_string()),
                            message: OverlayEvent::Select(id),
                        },
                        RowKind::Submenu { child_index } => {
                            let mut path = panel.path.clone();
                            path.push(child_index);
                            MenuItem::Action {
                                label: format!(
                                    "{} >",
                                    label_for_row(self.items.nodes(), &panel.path, row)
                                        .unwrap_or_else(|| "Submenu".to_string())
                                ),
                                message: OverlayEvent::OpenPath(path),
                            }
                        }
                        RowKind::Disabled => MenuItem::Disabled {
                            label: label_for_row(self.items.nodes(), &panel.path, row)
                                .unwrap_or_else(|| "Disabled".to_string()),
                        },
                        RowKind::Separator => MenuItem::Separator,
                    })
                    .collect::<Vec<_>>();
                (items, *rect)
            })
            .collect::<Vec<_>>();

        let overlay_widget = context_menu_overlay_panels(
            state.open.expect("checked"),
            panels,
            OverlayEvent::Close,
            OverlayEvent::Inert,
            self.style.clone(),
        );

        let structure_key: OverlayStructureKey = (runtime.panels.len(), state.open_path.clone());

        // `shell.invalidate_layout()` when `open_path` changes keeps overlay layout aligned with
        // the widget tree (avoids iced container `mouse_interaction` panics). When panel depth and
        // path are unchanged, `Tree::diff` preserves text/button subtree state so labels render;
        // on structure change, `Tree::new` realigns the stack.
        match state.last_overlay_structure.as_ref() {
            Some(prev) if *prev == structure_key => {
                state.overlay_tree.diff(overlay_widget.as_widget());
            }
            _ => {
                state.overlay_tree = Tree::new(overlay_widget.as_widget());
            }
        }

        state.last_overlay_structure = Some(structure_key);

        let bridge = OverlayBridge {
            widget: overlay_widget,
            tree: &mut state.overlay_tree,
            viewport: *viewport,
            open: &mut state.open,
            last_overlay_structure: &mut state.last_overlay_structure,
            open_path: &mut state.open_path,
            hover_path: &mut state.hover_path,
            hover_started_at: &mut state.hover_started_at,
            focused_indices: &mut state.focused_indices,
            on_close: self.on_close.clone(),
            on_select: self.on_select.clone(),
            close_on_select: self.close_on_select,
            submenu_open_mode: self.submenu_open_mode,
            menu_nodes: self.items.nodes(),
            style: self.style.clone(),
            viewport_size: state.viewport,
            submenu_hover_delay_ms: self.submenu_hover_delay_ms,
        };

        let element = overlay::Element::new(Box::new(bridge));

        match content_overlay {
            Some(content) => Some(overlay::Group::with_children(vec![content, element]).overlay()),
            None => Some(element),
        }
    }
}

struct OverlayBridge<'a, 'b, 'c, Message: Clone> {
    widget: Element<'b, OverlayEvent>,
    tree: &'a mut Tree,
    /// Overlay layout bounds (window/viewport space); matches `cursor.position()` in overlay updates.
    viewport: Rectangle,
    open: &'a mut Option<ContextMenuOpen>,
    last_overlay_structure: &'a mut Option<OverlayStructureKey>,
    open_path: &'a mut Vec<usize>,
    hover_path: &'a mut Vec<usize>,
    hover_started_at: &'a mut Option<Instant>,
    focused_indices: &'a mut Vec<usize>,
    on_close: Option<Message>,
    on_select: Option<Rc<dyn Fn(MenuItemId) -> Message + 'b>>,
    close_on_select: bool,
    submenu_open_mode: SubmenuOpenMode,
    menu_nodes: &'c [MenuNode],
    style: ContextMenuStyle,
    viewport_size: Size,
    submenu_hover_delay_ms: u16,
}

impl<Message: Clone> OverlayTrait<Message, Theme, iced::Renderer>
    for OverlayBridge<'_, '_, '_, Message>
{
    fn layout(&mut self, renderer: &iced::Renderer, bounds: Size) -> layout::Node {
        let limits = layout::Limits::new(Size::ZERO, bounds)
            .width(Length::Fill)
            .height(Length::Fill);
        self.widget.as_widget_mut().layout(self.tree, renderer, &limits)
    }

    fn update(
        &mut self,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &iced::Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
    ) {
        let mut local_messages = Vec::new();
        let mut local_shell = Shell::new(&mut local_messages);

        self.widget.as_widget_mut().update(
            self.tree,
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            &mut local_shell,
            &self.viewport,
        );

        for event in local_messages {
            match event {
                OverlayEvent::Close => {
                    let was_open = self.open.is_some();
                    *self.open = None;
                    *self.last_overlay_structure = None;
                    self.open_path.clear();
                    self.hover_path.clear();
                    *self.hover_started_at = None;
                    self.focused_indices.clear();
                    shell.invalidate_layout();
                    if was_open {
                        if let Some(message) = &self.on_close {
                            shell.publish(message.clone());
                        }
                    }
                    shell.capture_event();
                }
                OverlayEvent::Inert => {
                    shell.capture_event();
                }
                OverlayEvent::OpenPath(path) => {
                    if matches!(self.submenu_open_mode, SubmenuOpenMode::Click | SubmenuOpenMode::HoverAndClick)
                    {
                        *self.open_path = path;
                        shell.invalidate_layout();
                        shell.capture_event();
                    }
                }
                OverlayEvent::Select(id) => {
                    if let Some(map) = &self.on_select {
                        shell.publish(map(id));
                    }
                    if self.close_on_select {
                        let was_open = self.open.is_some();
                        *self.open = None;
                        *self.last_overlay_structure = None;
                        self.open_path.clear();
                        self.hover_path.clear();
                        *self.hover_started_at = None;
                        self.focused_indices.clear();
                        shell.invalidate_layout();
                        if was_open {
                            if let Some(message) = &self.on_close {
                                shell.publish(message.clone());
                            }
                        }
                    }
                    shell.capture_event();
                }
            }
        }

        if let Some(open) = *self.open {
            let now = Instant::now();
            let cursor_pos = cursor.position().unwrap_or(Point::ORIGIN);
            let open_path_before = self.open_path.clone();
            let needs_hover_redraw = poll_hover_submenu(
                event,
                now,
                cursor_pos,
                self.menu_nodes,
                self.open_path,
                self.hover_path,
                self.hover_started_at,
                open.at,
                self.viewport_size,
                &self.style,
                self.submenu_open_mode,
                self.submenu_hover_delay_ms,
            );
            if *self.open_path != open_path_before {
                shell.invalidate_layout();
            }
            if needs_hover_redraw {
                shell.request_redraw();
            }
        }
    }

    fn draw(
        &self,
        renderer: &mut iced::Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
    ) {
        self.widget
            .as_widget()
            .draw(self.tree, renderer, theme, style, layout, cursor, &self.viewport);
    }

    fn mouse_interaction(
        &self,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        self.widget
            .as_widget()
            .mouse_interaction(self.tree, layout, cursor, &self.viewport, renderer)
    }
}

struct RuntimeModel {
    panels: Vec<PanelModel>,
    rects: Vec<Rectangle>,
}

fn build_runtime(
    nodes: &[MenuNode],
    open_path: &[usize],
    anchor: Point,
    viewport: Size,
    style: &ContextMenuStyle,
) -> RuntimeModel {
    let panels = build_panels(nodes, open_path);
    let mut rects: Vec<Rectangle> = Vec::new();

    for (i, panel) in panels.iter().enumerate() {
        let items = panel_rows_to_dummy_items(panel);
        let height = estimate_panel_height(&items, style);
        let width = style.min_width.max(120.0);

        let mut point = if i == 0 {
            clamp_panel_anchor(anchor, width, height, viewport)
        } else {
            let parent = rects[i - 1];
            let mut candidate = Point::new(parent.x + parent.width + 4.0, parent.y);
            if candidate.x + width + 8.0 > viewport.width {
                candidate.x = (parent.x - width - 4.0).max(8.0);
            }
            clamp_panel_anchor(candidate, width, height, viewport)
        };

        if point.y + height > viewport.height - 8.0 {
            point.y = (viewport.height - height - 8.0).max(8.0);
        }

        rects.push(Rectangle::new(point, Size::new(width, height)));
    }

    RuntimeModel { panels, rects }
}

fn build_panels(nodes: &[MenuNode], open_path: &[usize]) -> Vec<PanelModel> {
    let mut out = Vec::new();
    let mut current_nodes = nodes;
    let mut current_path = Vec::new();

    out.push(PanelModel {
        path: current_path.clone(),
        rows: rows_for_nodes(current_nodes),
    });

    for &idx in open_path {
        let mut submenu_count = 0usize;
        let mut found = None;

        for node in current_nodes {
            if let MenuNode::Submenu { children, .. } = node {
                if submenu_count == idx {
                    found = Some(children.as_slice());
                    break;
                }
                submenu_count += 1;
            }
        }

        let Some(next_nodes) = found else {
            break;
        };

        current_path.push(idx);
        current_nodes = next_nodes;
        out.push(PanelModel {
            path: current_path.clone(),
            rows: rows_for_nodes(current_nodes),
        });
    }

    out
}

fn rows_for_nodes(nodes: &[MenuNode]) -> Vec<RowModel> {
    let mut rows = Vec::new();
    let mut submenu_index = 0usize;

    for node in nodes {
        match node {
            MenuNode::Action { id, enabled, .. } => {
                rows.push(RowModel {
                    kind: if *enabled {
                        RowKind::Action(*id)
                    } else {
                        RowKind::Disabled
                    },
                });
            }
            MenuNode::Separator => rows.push(RowModel {
                kind: RowKind::Separator,
            }),
            MenuNode::Submenu { .. } => {
                rows.push(RowModel {
                    kind: RowKind::Submenu {
                        child_index: submenu_index,
                    },
                });
                submenu_index += 1;
            }
        }
    }

    rows
}

fn panel_rows_to_dummy_items(panel: &PanelModel) -> Vec<MenuItem<OverlayEvent>> {
    panel
        .rows
        .iter()
        .map(|row| match row.kind {
            RowKind::Action(id) => MenuItem::Action {
                label: format!("{}", id),
                message: OverlayEvent::Select(id),
            },
            RowKind::Submenu { child_index } => MenuItem::Action {
                label: format!("submenu-{child_index}"),
                message: OverlayEvent::Inert,
            },
            RowKind::Disabled => MenuItem::Disabled {
                label: "disabled".to_string(),
            },
            RowKind::Separator => MenuItem::Separator,
        })
        .collect()
}

fn resolve_nodes_at_path<'a>(nodes: &'a [MenuNode], path: &[usize]) -> &'a [MenuNode] {
    let mut current = nodes;

    for &idx in path {
        let mut submenu_count = 0usize;
        let mut found = None;
        for node in current {
            if let MenuNode::Submenu { children, .. } = node {
                if submenu_count == idx {
                    found = Some(children.as_slice());
                    break;
                }
                submenu_count += 1;
            }
        }
        if let Some(next) = found {
            current = next;
        } else {
            break;
        }
    }

    current
}

fn label_for_row(nodes: &[MenuNode], panel_path: &[usize], row: &RowModel) -> Option<String> {
    let current = resolve_nodes_at_path(nodes, panel_path);
    let mut submenu_seen = 0usize;

    for node in current {
        match (node, &row.kind) {
            (MenuNode::Action { title, enabled, .. }, RowKind::Action(_)) if *enabled => {
                return Some(title.clone());
            }
            (MenuNode::Action { title, enabled, .. }, RowKind::Disabled) if !*enabled => {
                return Some(title.clone());
            }
            (MenuNode::Submenu { title, .. }, RowKind::Submenu { child_index })
                if submenu_seen == *child_index =>
            {
                return Some(title.clone());
            }
            (MenuNode::Submenu { .. }, RowKind::Submenu { .. }) => submenu_seen += 1,
            _ => {}
        }
    }

    None
}

fn hovered_submenu_path(
    panels: &[PanelModel],
    rects: &[Rectangle],
    cursor: Point,
    style: &ContextMenuStyle,
) -> Option<Vec<usize>> {
    for (panel, rect) in panels.iter().zip(rects.iter()) {
        if !rect.contains(cursor) {
            continue;
        }

        let local_y = cursor.y - rect.y - style.panel_padding;
        let mut y = 0.0;
        for row in &panel.rows {
            let h = match row.kind {
                RowKind::Separator => style.separator_height + 2.0 * style.separator_margin_vertical,
                _ => style.row_height,
            };
            if local_y >= y && local_y <= y + h {
                if let RowKind::Submenu { child_index } = row.kind {
                    let mut p = panel.path.clone();
                    p.push(child_index);
                    return Some(p);
                }
                return None;
            }
            y += h + style.row_spacing;
        }
    }

    None
}

fn ensure_focus_len(panels: &[PanelModel], focused_indices: &mut Vec<usize>) {
    if focused_indices.len() < panels.len() {
        focused_indices.resize(panels.len(), 0);
    } else if focused_indices.len() > panels.len() {
        focused_indices.truncate(panels.len());
    }

    for (i, panel) in panels.iter().enumerate() {
        if !is_focusable(panel, focused_indices[i]) {
            focused_indices[i] = first_focusable(panel).unwrap_or(0);
        }
    }
}

fn is_focusable(panel: &PanelModel, idx: usize) -> bool {
    panel
        .rows
        .get(idx)
        .map(|r| !matches!(r.kind, RowKind::Separator | RowKind::Disabled))
        .unwrap_or(false)
}

fn first_focusable(panel: &PanelModel) -> Option<usize> {
    panel.rows.iter().position(|r| !matches!(r.kind, RowKind::Separator | RowKind::Disabled))
}

fn move_focus(panel: &PanelModel, focus: &mut usize, dir: i32) {
    if panel.rows.is_empty() {
        return;
    }

    let mut idx = *focus as i32;
    for _ in 0..panel.rows.len() {
        idx = (idx + dir).rem_euclid(panel.rows.len() as i32);
        if !matches!(panel.rows[idx as usize].kind, RowKind::Separator | RowKind::Disabled) {
            *focus = idx as usize;
            return;
        }
    }
}

/// Pointer-driven submenu hover open. Call from the overlay so cursor and panel hit-testing use the same event layer.
/// `cursor` and panel rects from [`build_runtime`] are both in window/viewport coordinates (same as `cursor.position()` here).
fn poll_hover_submenu(
    event: &Event,
    now: Instant,
    cursor: Point,
    menu_nodes: &[MenuNode],
    open_path: &mut Vec<usize>,
    hover_path: &mut Vec<usize>,
    hover_started_at: &mut Option<Instant>,
    anchor: Point,
    viewport_size: Size,
    style: &ContextMenuStyle,
    mode: SubmenuOpenMode,
    delay_ms: u16,
) -> bool {
    if !matches!(mode, SubmenuOpenMode::Hover | SubmenuOpenMode::HoverAndClick) {
        return false;
    }

    let runtime = build_runtime(menu_nodes, open_path, anchor, viewport_size, style);

    if matches!(event, Event::Mouse(mouse::Event::CursorMoved { .. })) {
        let hovered = hovered_submenu_path(&runtime.panels, &runtime.rects, cursor, style).unwrap_or_default();

        if hovered != *hover_path {
            *hover_path = hovered;
            *hover_started_at = Some(now);
        }
    }

    if !hover_path.is_empty()
        && open_path.as_slice() != hover_path.as_slice()
        && should_open_hover(*hover_started_at, now, delay_ms)
    {
        *open_path = hover_path.clone();
    }

    !hover_path.is_empty() && open_path.as_slice() != hover_path.as_slice()
}

fn should_open_hover(started_at: Option<Instant>, now: Instant, delay_ms: u16) -> bool {
    started_at
        .map(|started| now.duration_since(started).as_millis() >= u128::from(delay_ms))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn nested_tree() -> Vec<MenuNode> {
        vec![
            MenuNode::Action {
                id: 1_u64.into(),
                title: "Copy".into(),
                enabled: true,
            },
            MenuNode::Separator,
            MenuNode::Submenu {
                title: "More".into(),
                children: vec![
                    MenuNode::Action {
                        id: 2_u64.into(),
                        title: "Rename".into(),
                        enabled: true,
                    },
                    MenuNode::Submenu {
                        title: "Share".into(),
                        children: vec![MenuNode::Action {
                            id: 3_u64.into(),
                            title: "Copy link".into(),
                            enabled: true,
                        }],
                    },
                ],
            },
        ]
    }

    #[test]
    fn builds_panel_per_open_depth() {
        let panels = build_panels(&nested_tree(), &[0, 0]);
        assert_eq!(panels.len(), 3);
        assert_eq!(panels[1].path, vec![0]);
        assert_eq!(panels[2].path, vec![0, 0]);
    }

    #[test]
    fn focus_skips_inert_rows() {
        let panel = PanelModel {
            path: vec![],
            rows: vec![
                RowModel {
                    kind: RowKind::Separator,
                },
                RowModel {
                    kind: RowKind::Disabled,
                },
                RowModel {
                    kind: RowKind::Action(1_u64.into()),
                },
            ],
        };
        let mut focus = 0;
        move_focus(&panel, &mut focus, 1);
        assert_eq!(focus, 2);
    }

    #[test]
    fn hover_path_detects_submenu_row() {
        let style = ContextMenuStyle::default();
        let runtime = build_runtime(&nested_tree(), &[], Point::new(50.0, 50.0), Size::new(800.0, 600.0), &style);
        let rect = runtime.rects[0];
        let cursor = Point::new(rect.x + 12.0, rect.y + style.panel_padding + style.row_height * 2.0);
        let path = hovered_submenu_path(&runtime.panels, &runtime.rects, cursor, &style);
        assert_eq!(path, Some(vec![0]));
    }

    #[test]
    fn open_mode_policy_matrix() {
        assert!(matches!(SubmenuOpenMode::Hover, SubmenuOpenMode::Hover));
        assert!(matches!(SubmenuOpenMode::Click, SubmenuOpenMode::Click));
        assert!(matches!(SubmenuOpenMode::HoverAndClick, SubmenuOpenMode::HoverAndClick));
    }

    #[test]
    fn hover_delay_gate_works() {
        let started = Instant::now();
        let now_before = started + std::time::Duration::from_millis(40);
        let now_after = started + std::time::Duration::from_millis(180);

        assert!(!should_open_hover(Some(started), now_before, 120));
        assert!(should_open_hover(Some(started), now_after, 120));
        assert!(!should_open_hover(None, now_after, 120));
    }

    #[test]
    fn poll_hover_promotes_without_extra_cursor_move() {
        let tree = nested_tree();
        let style = ContextMenuStyle::default();
        let mut open_path = Vec::new();
        let mut hover_path = vec![0usize];
        let started = Instant::now() - std::time::Duration::from_millis(300);
        let mut hover_started_at = Some(started);
        let anchor = Point::new(50.0, 50.0);
        let vp = Size::new(800.0, 600.0);
        let now = Instant::now();
        let ev = Event::Window(iced::window::Event::RedrawRequested(iced::time::Instant::now()));

        let pending = poll_hover_submenu(
            &ev,
            now,
            Point::ORIGIN,
            &tree,
            &mut open_path,
            &mut hover_path,
            &mut hover_started_at,
            anchor,
            vp,
            &style,
            SubmenuOpenMode::Hover,
            120,
        );

        assert_eq!(open_path, vec![0]);
        assert!(!pending);
    }

    #[test]
    fn poll_hover_click_mode_does_not_track() {
        let tree = nested_tree();
        let style = ContextMenuStyle::default();
        let mut open_path = Vec::new();
        let mut hover_path = vec![0usize];
        let mut hover_started_at = Some(Instant::now() - std::time::Duration::from_secs(10));
        let anchor = Point::new(50.0, 50.0);
        let vp = Size::new(800.0, 600.0);
        let now = Instant::now();
        let ev = Event::Window(iced::window::Event::RedrawRequested(iced::time::Instant::now()));

        poll_hover_submenu(
            &ev,
            now,
            Point::ORIGIN,
            &tree,
            &mut open_path,
            &mut hover_path,
            &mut hover_started_at,
            anchor,
            vp,
            &style,
            SubmenuOpenMode::Click,
            120,
        );

        assert!(open_path.is_empty());
    }
}
