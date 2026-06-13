//! Recursive context-menu overlay: root (`flyout_depth: None`) draws scrim + root panel;
//! nested instances (`flyout_depth: Some(d)`) match the former `SubmenuOverlay` depth `d`.

use std::marker::PhantomData;

use super::menu::{MenuItemId, MenuNode, MenuSpec};
use super::panel::PanelMetrics;
use super::style::{Catalog, ContextMenuStyle};

use super::panel::{Layout, draw_panel, layout_panel, row_geometries, row_index_at_panel_y};
use super::state::{
    ContextMenuState, SubmenuOpenMode, current_nodes, first_focusable, next_focusable,
    node_at_path, submenu_children, sync_open_path_for_focus,
};

use iced::advanced::layout;
use iced::advanced::overlay;
use iced::advanced::renderer;
use iced::advanced::svg;
use iced::advanced::text;
use iced::advanced::{Clipboard, Shell};
use iced::keyboard;
use iced::mouse;
use iced::touch;
use iced::{Color, Event, Point, Rectangle, Shadow, Size, Vector};

/// `flyout_depth: None` — root menu (scrim, `state.anchor`, keyboard nav).
/// `flyout_depth: Some(d)` — nested panel; same `d` as the old `SubmenuOverlay::depth`.
pub(crate) struct MenuOverlay<'a, 'b, Message, Theme, Renderer>
where
    Theme: Catalog,
    'b: 'a,
{
    pub(crate) state: &'a mut ContextMenuState,
    pub(crate) items: &'a MenuSpec<'b>,
    pub(crate) metrics: PanelMetrics,
    pub(crate) class: &'a Theme::Class<'b>,
    pub(crate) hotkey_label_color_override: Option<Color>,
    pub(crate) panel_shadow_override: Option<Shadow>,
    pub(crate) submenu_mode: SubmenuOpenMode,
    pub(crate) icons_enabled: bool,
    pub(crate) close_on_select: bool,
    pub(crate) on_close: Option<Message>,
    pub(crate) on_select: Option<&'a (dyn Fn(MenuItemId) -> Message + 'b)>,
    pub(crate) viewport: Rectangle,
    pub(crate) translation: Vector,
    pub(crate) flyout_depth: Option<usize>,
    /// Used when `flyout_depth` is `Some`; ignored for root (layout uses `state.anchor`).
    /// `x`=parent left edge, `y`=row top, `width`=parent panel width; `height` unused.
    pub(crate) anchor: Rectangle,
    pub(crate) _marker: PhantomData<(Theme, Renderer)>,
}

impl<'a, 'b, Message: Clone, Theme: Catalog, Renderer: text::Renderer<Font = iced::Font> + svg::Renderer>
    MenuOverlay<'a, 'b, Message, Theme, Renderer>
where
    'b: 'a,
{
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        state: &'a mut ContextMenuState,
        items: &'a MenuSpec<'b>,
        metrics: PanelMetrics,
        class: &'a Theme::Class<'b>,
        hotkey_label_color_override: Option<Color>,
        panel_shadow_override: Option<Shadow>,
        submenu_mode: SubmenuOpenMode,
        icons_enabled: bool,
        close_on_select: bool,
        on_close: Option<Message>,
        on_select: Option<&'a (dyn Fn(MenuItemId) -> Message + 'b)>,
        viewport: Rectangle,
        translation: Vector,
        flyout_depth: Option<usize>,
        anchor: Rectangle,
    ) -> Self {
        Self {
            state,
            items,
            metrics,
            class,
            hotkey_label_color_override,
            panel_shadow_override,
            submenu_mode,
            icons_enabled,
            close_on_select,
            on_close,
            on_select,
            viewport,
            translation,
            flyout_depth,
            anchor,
            _marker: PhantomData,
        }
    }

    fn resolve_style(&self, theme: &Theme) -> ContextMenuStyle {
        super::widget::resolve_menu_style(
            theme,
            self.class,
            self.hotkey_label_color_override,
            self.panel_shadow_override,
        )
    }

    fn write_submenu_anchor_for_next_row(
        state: &mut ContextMenuState,
        nodes: &[MenuNode<'_>],
        metrics: &PanelMetrics,
        panel_bounds: Rectangle,
        panel_w: f32,
        open_path: &[usize],
        next_index: usize,
    ) {
        if open_path.len() <= next_index {
            return;
        }
        let ri = open_path[next_index];
        let geoms = row_geometries(nodes, metrics);
        if let Some(g) = geoms.iter().find(|g| g.node_idx == ri) {
            let row_top =
                panel_bounds.y + metrics.panel_padding + metrics.border_width + g.y_offset;
            let rect = Rectangle {
                x: panel_bounds.x,
                y: row_top,
                width: panel_w,
                height: metrics.row_height + metrics.row_spacing,
            };
            if state.submenu_anchors.len() <= next_index {
                state.submenu_anchors.resize(next_index + 1, Rectangle::default());
            }
            state.submenu_anchors[next_index] = rect;
        }
    }

    fn activate_row(
        state: &mut ContextMenuState,
        submenu_mode: SubmenuOpenMode,
        close_on_select: bool,
        path: &[usize],
        nodes: &[MenuNode<'_>],
        idx: usize,
        on_close: &Option<Message>,
        on_select: Option<&dyn Fn(MenuItemId) -> Message>,
        shell: &mut Shell<'_, Message>,
    ) {
        let Some(node) = nodes.get(idx) else {
            return;
        };
        match node {
            MenuNode::Action {
                id, enabled: true, ..
            } => {
                if let Some(f) = on_select {
                    shell.publish(f(*id));
                }
                if close_on_select {
                    state.close();
                    if let Some(m) = on_close.clone() {
                        shell.publish(m);
                    }
                }
                shell.capture_event();
                shell.request_redraw();
            }
            MenuNode::Submenu { .. } => {
                if submenu_mode == SubmenuOpenMode::Click {
                    if state.open_path.starts_with(path) && state.open_path.len() == path.len() {
                        state.open_path.truncate(path.len().saturating_sub(1));
                    } else {
                        state.open_path = path.to_vec();
                    }
                    shell.capture_event();
                    shell.request_redraw();
                }
            }
            _ => {}
        }
    }

    fn handle_escape(
        state: &mut ContextMenuState,
        on_close: &Option<Message>,
        shell: &mut Shell<'_, Message>,
    ) {
        state.close();
        if let Some(m) = on_close.clone() {
            shell.publish(m);
        }
        shell.capture_event();
        shell.request_redraw();
    }

    fn update_panel_pointer(
        state: &mut ContextMenuState,
        items: &'a MenuSpec<'b>,
        metrics: &PanelMetrics,
        submenu_mode: SubmenuOpenMode,
        close_on_select: bool,
        on_close: &Option<Message>,
        on_select: Option<&dyn Fn(MenuItemId) -> Message>,
        event: &Event,
        panel_layout: Layout<'_>,
        cursor: mouse::Cursor,
        shell: &mut Shell<'_, Message>,
        nodes: &[MenuNode<'_>],
        prefix_path: &[usize],
    ) {
        if let Some(p) = cursor.position_in(panel_layout.bounds()) {
            if let Event::Mouse(mouse::Event::CursorMoved { .. }) = event {
                if let Some(idx) = row_index_at_panel_y(nodes, metrics, p.y) {
                    let mut new_focus = prefix_path.to_vec();
                    new_focus.push(idx);
                    state.focus_path = new_focus.clone();
                    sync_open_path_for_focus(state, items, submenu_mode, &new_focus);
                    shell.request_redraw();
                }
            }

            if let Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
            | Event::Touch(touch::Event::FingerPressed { .. }) = event
            {
                if let Some(idx) = row_index_at_panel_y(nodes, metrics, p.y) {
                    let mut path = prefix_path.to_vec();
                    path.push(idx);
                    Self::activate_row(
                        state,
                        submenu_mode,
                        close_on_select,
                        &path,
                        nodes,
                        idx,
                        on_close,
                        on_select,
                        shell,
                    );
                }
            }
        }
    }

    fn handle_keyboard_nav(
        state: &mut ContextMenuState,
        items: &'a MenuSpec<'b>,
        submenu_mode: SubmenuOpenMode,
        event: &Event,
        shell: &mut Shell<'_, Message>,
        on_close: &Option<Message>,
        on_select: Option<&dyn Fn(MenuItemId) -> Message>,
        close_on_select: bool,
    ) {
        let Event::Keyboard(keyboard::Event::KeyPressed { key, .. }) = event else {
            return;
        };
        let current = current_nodes(items.nodes(), &state.focus_path);
        let last_idx = *state.focus_path.last().unwrap_or(&0);

        match key.as_ref() {
            keyboard::Key::Named(keyboard::key::Named::ArrowDown) => {
                if let Some(i) = next_focusable(current, last_idx, 1) {
                    if state.focus_path.is_empty() {
                        state.focus_path.push(i);
                    } else {
                        *state.focus_path.last_mut().unwrap() = i;
                    }
                    let fp = state.focus_path.clone();
                    sync_open_path_for_focus(state, items, submenu_mode, &fp);
                    shell.capture_event();
                    shell.request_redraw();
                }
            }
            keyboard::Key::Named(keyboard::key::Named::ArrowUp) => {
                if let Some(i) = next_focusable(current, last_idx, -1) {
                    if state.focus_path.is_empty() {
                        state.focus_path.push(i);
                    } else {
                        *state.focus_path.last_mut().unwrap() = i;
                    }
                    let fp = state.focus_path.clone();
                    sync_open_path_for_focus(state, items, submenu_mode, &fp);
                    shell.capture_event();
                    shell.request_redraw();
                }
            }
            keyboard::Key::Named(keyboard::key::Named::ArrowRight) => {
                let n = node_at_path(items.nodes(), &state.focus_path);
                if let Some(MenuNode::Submenu { .. }) = n {
                    let mut p = state.focus_path.clone();
                    state.open_path = p.clone();
                    if let Some(children) = submenu_children(items.nodes(), &p) {
                        if let Some(ci) = first_focusable(children, None) {
                            p.push(ci);
                            state.focus_path = p;
                        }
                    }
                    shell.capture_event();
                    shell.request_redraw();
                }
            }
            keyboard::Key::Named(keyboard::key::Named::ArrowLeft) => {
                if state.focus_path.len() > 1 {
                    state.focus_path.pop();
                    state
                        .open_path
                        .truncate(state.focus_path.len().saturating_sub(1));
                    shell.capture_event();
                    shell.request_redraw();
                }
            }
            keyboard::Key::Named(keyboard::key::Named::Enter) => {
                let path = state.focus_path.clone();
                if let Some(&idx) = path.last() {
                    let parent = if path.len() == 1 {
                        items.nodes()
                    } else {
                        submenu_children(items.nodes(), &path[..path.len() - 1]).unwrap_or(&[])
                    };
                    Self::activate_row(
                        state,
                        submenu_mode,
                        close_on_select,
                        &path,
                        parent,
                        idx,
                        on_close,
                        on_select,
                        shell,
                    );
                }
            }
            _ => {}
        }
    }
}

impl<
        'a,
        'b,
        Message: Clone,
        Theme: Catalog,
        Renderer: text::Renderer<Font = iced::Font> + svg::Renderer,
    > overlay::Overlay<Message, Theme, Renderer> for MenuOverlay<'a, 'b, Message, Theme, Renderer>
where
    'b: 'a,
{
    fn layout(&mut self, renderer: &Renderer, bounds: Size) -> layout::Node {
        match self.flyout_depth {
            None => {
                let nodes = self.items.nodes();
                let (panel_node, panel_w, _panel_h) = layout_panel(
                    renderer,
                    &self.metrics,
                    nodes,
                    self.state.anchor,
                    None,
                    bounds,
                    self.icons_enabled,
                    0.0,
                    0.0,
                );

                self.state.submenu_anchors.clear();
                if !self.state.open_path.is_empty() {
                    let open_path = self.state.open_path.clone();
                    let pb = panel_node.bounds();
                    Self::write_submenu_anchor_for_next_row(
                        self.state,
                        nodes,
                        &self.metrics,
                        pb,
                        panel_w,
                        &open_path,
                        0,
                    );
                }

                let scrim = layout::Node::new(bounds);
                layout::Node::with_children(bounds, vec![scrim, panel_node])
            }
            Some(depth) => {
                if self.state.open_path.len() <= depth {
                    return layout::Node::new(bounds);
                }
                let path = &self.state.open_path[0..=depth];
                let Some(nodes) = submenu_children(self.items.nodes(), path) else {
                    return layout::Node::new(bounds);
                };
                let anchor_pt = Point::new(
                    self.anchor.x + self.anchor.width - self.metrics.border_width,
                    self.anchor.y,
                );
                let (panel_node, panel_w, _panel_h) = layout_panel(
                    renderer,
                    &self.metrics,
                    nodes,
                    anchor_pt,
                    Some(self.anchor.x),
                    bounds,
                    self.icons_enabled,
                    self.metrics.submenu_flyout_overlap,
                    self.anchor.height,
                );

                let next = depth + 1;
                if self.state.open_path.len() > next {
                    let open_path = self.state.open_path.clone();
                    let pb = panel_node.bounds();
                    Self::write_submenu_anchor_for_next_row(
                        self.state,
                        nodes,
                        &self.metrics,
                        pb,
                        panel_w,
                        &open_path,
                        next,
                    );
                }

                layout::Node::with_children(bounds, vec![panel_node])
            }
        }
    }

    fn update(
        &mut self,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _renderer: &Renderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
    ) {
        if let Event::Keyboard(keyboard::Event::KeyPressed {
            key: keyboard::Key::Named(keyboard::key::Named::Escape),
            ..
        }) = event
        {
            Self::handle_escape(self.state, &self.on_close, shell);
            return;
        }

        match self.flyout_depth {
            None => {
                let nodes = self.items.nodes();
                let mut children = layout.children();
                let scrim_layout = children.next();
                let panel_layout = children.next();
                if let (Some(sl), Some(pl)) = (scrim_layout, panel_layout) {
                    if let Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
                    | Event::Touch(touch::Event::FingerPressed { .. }) = event
                    {
                        if let Some(p) = cursor.position() {
                            if sl.bounds().contains(p) && !pl.bounds().contains(p) {
                                Self::handle_escape(self.state, &self.on_close, shell);
                                return;
                            }
                        }
                    }
                    Self::update_panel_pointer(
                        self.state,
                        self.items,
                        &self.metrics,
                        self.submenu_mode,
                        self.close_on_select,
                        &self.on_close,
                        self.on_select,
                        event,
                        pl,
                        cursor,
                        shell,
                        nodes,
                        &[],
                    );
                }
                Self::handle_keyboard_nav(
                    self.state,
                    self.items,
                    self.submenu_mode,
                    event,
                    shell,
                    &self.on_close,
                    self.on_select,
                    self.close_on_select,
                );
            }
            Some(depth) => {
                if self.state.open_path.len() <= depth {
                    return;
                }
                let path = &self.state.open_path[0..=depth];
                let Some(nodes) = submenu_children(self.items.nodes(), path) else {
                    return;
                };
                let prefix: Vec<_> = self.state.open_path[0..=depth].to_vec();
                if let Some(pl) = layout.children().next() {
                    Self::update_panel_pointer(
                        self.state,
                        self.items,
                        &self.metrics,
                        self.submenu_mode,
                        self.close_on_select,
                        &self.on_close,
                        self.on_select,
                        event,
                        pl,
                        cursor,
                        shell,
                        nodes,
                        &prefix,
                    );
                }
            }
        }
    }

    fn draw(
        &self,
        renderer: &mut Renderer,
        theme: &Theme,
        _theme_style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
    ) {
        let style = self.resolve_style(theme);
        match self.flyout_depth {
            None => {
                let mut children = layout.children();
                let scrim_l = children.next();
                let panel_l = children.next();
                if let Some(sl) = scrim_l {
                    renderer.fill_quad(
                        renderer::Quad {
                            bounds: sl.bounds(),
                            ..renderer::Quad::default()
                        },
                        style.dismiss_scrim,
                    );
                }
                if let Some(pl) = panel_l {
                    draw_panel(
                        renderer,
                        &self.metrics,
                        &style,
                        self.items.nodes(),
                        pl,
                        cursor,
                        &self.state.focus_path,
                        &[],
                        &self.state.open_path,
                        layout.bounds(),
                        0,
                        self.icons_enabled,
                    );
                }
            }
            Some(depth) => {
                if self.state.open_path.len() <= depth {
                    return;
                }
                let path = &self.state.open_path[0..=depth];
                let Some(nodes) = submenu_children(self.items.nodes(), path) else {
                    return;
                };
                if let Some(pl) = layout.children().next() {
                    draw_panel(
                        renderer,
                        &self.metrics,
                        &style,
                        nodes,
                        pl,
                        cursor,
                        &self.state.focus_path,
                        path,
                        &self.state.open_path,
                        layout.bounds(),
                        depth,
                        self.icons_enabled,
                    );
                }
            }
        }
    }

    fn mouse_interaction(
        &self,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _renderer: &Renderer,
    ) -> mouse::Interaction {
        match self.flyout_depth {
            None => {
                if cursor
                    .position()
                    .is_some_and(|p| layout.bounds().contains(p))
                {
                    mouse::Interaction::Pointer
                } else {
                    mouse::Interaction::None
                }
            }
            Some(_) => {
                if let Some(pl) = layout.children().next() {
                    if cursor.position().is_some_and(|p| pl.bounds().contains(p)) {
                        return mouse::Interaction::Pointer;
                    }
                }
                mouse::Interaction::None
            }
        }
    }

    fn overlay<'c>(
        &'c mut self,
        _layout: Layout<'c>,
        _renderer: &Renderer,
    ) -> Option<overlay::Element<'c, Message, Theme, Renderer>> {
        match self.flyout_depth {
            None => {
                if self.state.open_path.is_empty() {
                    return None;
                }
                let anchor = self.state.submenu_anchors.get(0).copied()?;
                Some(overlay::Element::new(Box::new(MenuOverlay::new(
                    self.state,
                    self.items,
                    self.metrics,
                    self.class,
                    self.hotkey_label_color_override,
                    self.panel_shadow_override,
                    self.submenu_mode,
                    self.icons_enabled,
                    self.close_on_select,
                    self.on_close.clone(),
                    self.on_select,
                    self.viewport,
                    self.translation,
                    Some(0),
                    anchor,
                ))))
            }
            Some(depth) => {
                let next = depth + 1;
                if self.state.open_path.len() <= next {
                    return None;
                }
                let anchor = self.state.submenu_anchors.get(next).copied()?;
                Some(overlay::Element::new(Box::new(MenuOverlay::new(
                    self.state,
                    self.items,
                    self.metrics,
                    self.class,
                    self.hotkey_label_color_override,
                    self.panel_shadow_override,
                    self.submenu_mode,
                    self.icons_enabled,
                    self.close_on_select,
                    self.on_close.clone(),
                    self.on_select,
                    self.viewport,
                    self.translation,
                    Some(next),
                    anchor,
                ))))
            }
        }
    }

    fn index(&self) -> f32 {
        match self.flyout_depth {
            None => 1.0,
            Some(d) => 10.0 + d as f32,
        }
    }
}
