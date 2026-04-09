//! Context menu widget and nested overlays.

use crate::menu::{MenuItemId, MenuNode, MenuSpec};
use crate::style::ContextMenuStyle;

use iced::advanced::layout;
use iced::advanced::overlay;
use iced::advanced::renderer;
use iced::advanced::text::{self, Paragraph};
use iced::advanced::widget::tree::{self, Tree};
use iced::advanced::widget::Widget;
use iced::advanced::{Clipboard, Shell};
use iced::alignment;
use iced::border;
use iced::keyboard;
use iced::mouse;
use iced::touch;
use iced::time::{Duration as IcedDuration, Instant};
use iced::{
    Element, Event, Length, Pixels, Point, Rectangle, Size, Vector,
};

use std::marker::PhantomData;

const SUBMENU_CHEVRON: &str = "›";
const SUBMENU_CHEVRON_WIDTH: f32 = 18.0;

/// How nested submenus open.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SubmenuOpenMode {
    /// Open as soon as the pointer enters the submenu row.
    #[default]
    Hover,
    /// Open after the pointer rests on the row for [`ContextMenu::submenu_hover_delay_ms`].
    HoverDelayed,
    /// Open when the submenu row is clicked.
    Click,
}

/// Persistent state for [`ContextMenu`], stored in the widget [`Tree`].
#[derive(Debug, Clone)]
pub struct ContextMenuState {
    pub open: bool,
    pub anchor: Point,
    /// Keyboard / logical focus: indices from root through each nested panel.
    pub focus_path: Vec<usize>,
    /// Open submenu chain: `open_path[0]` is a root row index, etc.
    pub open_path: Vec<usize>,
    /// Pending delayed submenu (`HoverDelayed`).
    pub submenu_delay: Option<(Vec<usize>, Instant)>,
    /// Anchor for flyout at depth `d` (`submenu_anchors[d]` = top-left of that flyout panel).
    pub submenu_anchors: Vec<Point>,
}

impl Default for ContextMenuState {
    fn default() -> Self {
        Self {
            open: false,
            anchor: Point::ORIGIN,
            focus_path: Vec::new(),
            open_path: Vec::new(),
            submenu_delay: None,
            submenu_anchors: Vec::new(),
        }
    }
}

impl ContextMenuState {
    fn reset_interaction(&mut self) {
        self.focus_path.clear();
        self.open_path.clear();
        self.submenu_delay = None;
        self.submenu_anchors.clear();
    }

    fn close(&mut self) {
        self.open = false;
        self.reset_interaction();
    }

    fn ensure_focus(&mut self, nodes: &[MenuNode]) {
        if self.focus_path.is_empty() {
            if let Some(i) = first_focusable(nodes, None) {
                self.focus_path.push(i);
            }
        }
    }
}

fn first_focusable(nodes: &[MenuNode], skip: Option<usize>) -> Option<usize> {
    for (i, n) in nodes.iter().enumerate() {
        if skip == Some(i) {
            continue;
        }
        match n {
            MenuNode::Separator => {}
            MenuNode::Action { enabled: false, .. } => {}
            _ => return Some(i),
        }
    }
    None
}

fn next_focusable(nodes: &[MenuNode], from: usize, dir: isize) -> Option<usize> {
    if nodes.is_empty() {
        return None;
    }
    let len = nodes.len() as isize;
    let mut i = from as isize;
    for _ in 0..nodes.len() {
        i = (i + dir).rem_euclid(len);
        let ui = i as usize;
        match &nodes[ui] {
            MenuNode::Separator => continue,
            MenuNode::Action { enabled: false, .. } => continue,
            _ => return Some(ui),
        }
    }
    None
}

fn submenu_children<'a>(nodes: &'a [MenuNode], path: &[usize]) -> Option<&'a [MenuNode]> {
    let mut current = nodes;
    for (d, &idx) in path.iter().enumerate() {
        let node = current.get(idx)?;
        match node {
            MenuNode::Submenu { children, .. } => {
                if d + 1 == path.len() {
                    return Some(children.as_slice());
                }
                current = children.as_slice();
            }
            _ => return None,
        }
    }
    None
}

fn node_at_path<'a>(nodes: &'a [MenuNode], path: &[usize]) -> Option<&'a MenuNode> {
    let mut current = nodes;
    for (d, &idx) in path.iter().enumerate() {
        let node = current.get(idx)?;
        if d + 1 == path.len() {
            return Some(node);
        }
        match node {
            MenuNode::Submenu { children, .. } => current = children.as_slice(),
            _ => return None,
        }
    }
    None
}

fn current_nodes<'a>(root: &'a [MenuNode], focus_path: &[usize]) -> &'a [MenuNode] {
    if focus_path.len() <= 1 {
        return root;
    }
    submenu_children(root, &focus_path[..focus_path.len() - 1]).unwrap_or(root)
}

fn sync_open_path_for_focus<Message>(
    state: &mut ContextMenuState,
    items: &MenuSpec,
    mode: SubmenuOpenMode,
    hover_delay: IcedDuration,
    focus: &[usize],
    shell: &mut Shell<'_, Message>,
) {
    state.submenu_delay = None;
    if focus.is_empty() {
        state.open_path.clear();
        return;
    }

    let root = items.nodes();
    let mut open = Vec::new();
    let mut cur = root;

    for depth in 0..focus.len() {
        let idx = focus[depth];
        let Some(node) = cur.get(idx) else {
            break;
        };
        let is_last = depth + 1 == focus.len();

        if !is_last {
            if let MenuNode::Submenu { children, .. } = node {
                open.push(idx);
                cur = children;
            } else {
                break;
            }
        } else {
            match (node, mode) {
                (MenuNode::Submenu { .. }, SubmenuOpenMode::Hover) => {
                    open.push(idx);
                }
                (MenuNode::Submenu { .. }, SubmenuOpenMode::HoverDelayed) => {
                    state.submenu_delay = Some((focus.to_vec(), Instant::now()));
                    shell.request_redraw_at(Instant::now() + hover_delay);
                    state.open_path = open;
                    return;
                }
                _ => {}
            }
        }
    }

    state.open_path = open;
}

struct RowGeom {
    y_offset: f32,
    height: f32,
    node_idx: usize,
}

fn row_geometries(nodes: &[MenuNode], style: &ContextMenuStyle) -> Vec<RowGeom> {
    let mut out = Vec::new();
    let mut y = 0.0_f32;
    for (node_idx, node) in nodes.iter().enumerate() {
        let h = match node {
            MenuNode::Separator => {
                style.separator_margin_vertical * 2.0 + style.separator_height
            }
            _ => style.row_height + style.row_spacing,
        };
        out.push(RowGeom {
            y_offset: y,
            height: h,
            node_idx,
        });
        y += h;
    }
    out
}

fn panel_height(geoms: &[RowGeom]) -> f32 {
    geoms.last().map(|g| g.y_offset + g.height).unwrap_or(0.0)
}

fn row_index_at_y(nodes: &[MenuNode], style: &ContextMenuStyle, y: f32) -> Option<usize> {
    let geoms = row_geometries(nodes, style);
    for g in geoms {
        if y >= g.y_offset && y < g.y_offset + g.height {
            return Some(g.node_idx);
        }
    }
    None
}

fn measure_label_width<Renderer: text::Renderer>(
    renderer: &Renderer,
    style: &ContextMenuStyle,
    label: &str,
) -> f32 {
    let size = Pixels(style.label_size);
    let line_height = text::LineHeight::default();
    let text = text::Text {
        content: label,
        bounds: Size::new(f32::INFINITY, style.row_height),
        size,
        line_height,
        font: renderer.default_font(),
        align_x: text::Alignment::Left,
        align_y: alignment::Vertical::Top,
        shaping: text::Shaping::default(),
        wrapping: text::Wrapping::None,
    };
    <<Renderer as text::Renderer>::Paragraph as Paragraph>::with_text(text).min_width()
}

fn panel_content_width<Renderer: text::Renderer>(
    renderer: &Renderer,
    style: &ContextMenuStyle,
    nodes: &[MenuNode],
) -> f32 {
    let mut w = style.min_width;
    for node in nodes {
        let label = match node {
            MenuNode::Action { title, .. } => title.as_str(),
            MenuNode::Submenu { title, .. } => title.as_str(),
            MenuNode::Separator => continue,
        };
        let lw = measure_label_width(renderer, style, label);
        let extra = match node {
            MenuNode::Submenu { .. } => SUBMENU_CHEVRON_WIDTH,
            _ => 0.0,
        };
        w = w.max(lw + style.panel_padding * 2.0 + extra);
    }
    w
}

fn layout_panel<Renderer: text::Renderer>(
    renderer: &Renderer,
    style: &ContextMenuStyle,
    nodes: &[MenuNode],
    anchor: Point,
    viewport: Size,
) -> (layout::Node, f32, f32) {
    let width = panel_content_width(renderer, style, nodes);
    let geoms = row_geometries(nodes, style);
    let content_h = panel_height(&geoms);
    let border = style.border_width * 2.0;
    let panel_w = width + border;
    let panel_h = content_h + border + style.panel_padding * 2.0;

    let space_right = viewport.width - anchor.x;
    let space_left = anchor.x;
    let place_right = space_right >= panel_w || space_right >= space_left;
    let x = if place_right {
        anchor.x
    } else {
        anchor.x - panel_w
    }
    .clamp(0.0, (viewport.width - panel_w).max(0.0));

    let space_below = viewport.height - anchor.y;
    let space_above = anchor.y;
    let y = if space_below >= panel_h || space_below >= space_above {
        anchor.y.clamp(0.0, (viewport.height - panel_h).max(0.0))
    } else {
        (anchor.y - panel_h)
            .clamp(0.0, (viewport.height - panel_h).max(0.0))
    };

    let row_nodes: Vec<layout::Node> = geoms
        .iter()
        .map(|g| {
            layout::Node::new(Size::new(
                width,
                g.height - if matches!(nodes[g.node_idx], MenuNode::Separator) {
                    0.0
                } else {
                    style.row_spacing
                },
            ))
            .move_to(Point::new(
                style.panel_padding + style.border_width,
                style.panel_padding + style.border_width + g.y_offset,
            ))
        })
        .collect();

    let inner = layout::Node::with_children(
        Size::new(width, content_h + style.panel_padding * 2.0 + border),
        row_nodes,
    );

    let panel = layout::Node::with_children(Size::new(panel_w, panel_h), vec![inner]).move_to(Point::new(x, y));

    (panel, panel_w, panel_h)
}

fn draw_panel<Renderer: text::Renderer>(
    renderer: &mut Renderer,
    style: &ContextMenuStyle,
    nodes: &[MenuNode],
    layout: Layout<'_>,
    cursor: mouse::Cursor,
    focus_path: &[usize],
    prefix_path: &[usize],
    clip_bounds: Rectangle,
    depth: usize,
) where
    Renderer: text::Renderer,
{
    let bounds = layout.bounds();
    renderer.fill_quad(
        renderer::Quad {
            bounds,
            border: style.panel_border(),
            shadow: style.panel_shadow(),
            ..renderer::Quad::default()
        },
        style.panel_background,
    );

    let geoms = row_geometries(nodes, style);
    let row_layouts: Vec<_> = layout.children().collect();
    let inner = row_layouts.first();
    let row_lays: Vec<_> = inner
        .map(|l| l.children().collect())
        .unwrap_or_default();

    let text_size = Pixels(style.label_size);
    let line_height = text::LineHeight::default();
    let font = renderer.default_font();

    for g in &geoms {
        let Some(rl) = row_lays.get(g.node_idx) else {
            continue;
        };
        let row_bounds = rl.bounds();
        let node = &nodes[g.node_idx];

        let mut row_path = prefix_path.to_vec();
        row_path.push(g.node_idx);
        let is_focused = focus_path == row_path.as_slice();

        let is_hover = cursor
            .position()
            .is_some_and(|p| row_bounds.contains(p))
            && !matches!(node, MenuNode::Separator);

        let pressed = false;

        if is_focused || is_hover {
            if !matches!(node, MenuNode::Separator) {
                renderer.fill_quad(
                    renderer::Quad {
                        bounds: Rectangle {
                            x: row_bounds.x + style.border_width,
                            y: row_bounds.y,
                            width: row_bounds.width - style.border_width * 2.0,
                            height: row_bounds.height,
                        },
                        border: border::rounded(4.0),
                        ..renderer::Quad::default()
                    },
                    if pressed {
                        style.row_pressed_background
                    } else {
                        style.row_hover_background
                    },
                );
            }
        }

        match node {
            MenuNode::Separator => {
                let y = row_bounds.center_y();
                renderer.fill_quad(
                    renderer::Quad {
                        bounds: Rectangle {
                            x: row_bounds.x + style.panel_padding,
                            y: y - style.separator_height * 0.5,
                            width: row_bounds.width - style.panel_padding * 2.0,
                            height: style.separator_height,
                        },
                        ..renderer::Quad::default()
                    },
                    style.separator_color,
                );
            }
            MenuNode::Action {
                title,
                enabled,
                ..
            } => {
                let color = if *enabled {
                    style.label_color
                } else {
                    style.disabled_color
                };
                renderer.fill_text(
                    text::Text {
                        content: title.clone(),
                        bounds: Size::new(f32::INFINITY, row_bounds.height),
                        size: text_size,
                        line_height,
                        font,
                        align_x: text::Alignment::Left,
                        align_y: alignment::Vertical::Center,
                        shaping: text::Shaping::default(),
                        wrapping: text::Wrapping::None,
                    },
                    Point::new(row_bounds.x + style.panel_padding, row_bounds.center_y()),
                    color,
                    clip_bounds,
                );
            }
            MenuNode::Submenu { title, .. } => {
                renderer.fill_text(
                    text::Text {
                        content: title.clone(),
                        bounds: Size::new(f32::INFINITY, row_bounds.height),
                        size: text_size,
                        line_height,
                        font,
                        align_x: text::Alignment::Left,
                        align_y: alignment::Vertical::Center,
                        shaping: text::Shaping::default(),
                        wrapping: text::Wrapping::None,
                    },
                    Point::new(row_bounds.x + style.panel_padding, row_bounds.center_y()),
                    style.label_color,
                    clip_bounds,
                );
                renderer.fill_text(
                    text::Text {
                        content: SUBMENU_CHEVRON.to_string(),
                        bounds: Size::new(f32::INFINITY, row_bounds.height),
                        size: text_size,
                        line_height,
                        font,
                        align_x: text::Alignment::Right,
                        align_y: alignment::Vertical::Center,
                        shaping: text::Shaping::default(),
                        wrapping: text::Wrapping::None,
                    },
                    Point::new(row_bounds.x + row_bounds.width - style.panel_padding, row_bounds.center_y()),
                    style.label_color,
                    clip_bounds,
                );
            }
        }
    }

    let _ = depth;
}

type Layout<'a> = iced::advanced::Layout<'a>;

/// Right-click wrapper that shows a [`MenuSpec`] in an overlay.
pub struct ContextMenu<'a, Message, Theme = iced::Theme, Renderer = iced::Renderer> {
    content: Element<'a, Message, Theme, Renderer>,
    items: MenuSpec,
    style: ContextMenuStyle,
    submenu_mode: SubmenuOpenMode,
    submenu_hover_delay: IcedDuration,
    close_on_select: bool,
    on_open: Option<Message>,
    on_close: Option<Message>,
    on_select: Option<Box<dyn Fn(MenuItemId) -> Message + 'a>>,
}

impl<'a, Message, Theme, Renderer> ContextMenu<'a, Message, Theme, Renderer> {
    pub fn new(content: impl Into<Element<'a, Message, Theme, Renderer>>) -> Self {
        Self {
            content: content.into(),
            items: MenuSpec::default(),
            style: ContextMenuStyle::default(),
            submenu_mode: SubmenuOpenMode::default(),
            submenu_hover_delay: IcedDuration::from_millis(180),
            close_on_select: true,
            on_open: None,
            on_close: None,
            on_select: None,
        }
    }

    pub fn items(mut self, spec: MenuSpec) -> Self {
        self.items = spec;
        self
    }

    pub fn style(mut self, style: ContextMenuStyle) -> Self {
        self.style = style;
        self
    }

    pub fn submenu_open_mode(mut self, mode: SubmenuOpenMode) -> Self {
        self.submenu_mode = mode;
        self
    }

    pub fn submenu_hover_delay_ms(mut self, ms: u64) -> Self {
        self.submenu_hover_delay = IcedDuration::from_millis(ms);
        self
    }

    pub fn close_on_select(mut self, close: bool) -> Self {
        self.close_on_select = close;
        self
    }

    pub fn on_open(mut self, msg: Message) -> Self {
        self.on_open = Some(msg);
        self
    }

    pub fn on_close(mut self, msg: Message) -> Self {
        self.on_close = Some(msg);
        self
    }

    pub fn on_select(mut self, f: impl Fn(MenuItemId) -> Message + 'a) -> Self {
        self.on_select = Some(Box::new(f));
        self
    }
}

impl<'a, Message, Theme, Renderer> From<ContextMenu<'a, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: Clone + 'a,
    Theme: 'a,
    Renderer: 'a + text::Renderer,
{
    fn from(menu: ContextMenu<'a, Message, Theme, Renderer>) -> Self {
        Element::new(menu)
    }
}

impl<'a, Message: Clone, Theme, Renderer> Widget<Message, Theme, Renderer>
    for ContextMenu<'a, Message, Theme, Renderer>
where
    Renderer: text::Renderer,
{
    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.content
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits)
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
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

    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<ContextMenuState>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(ContextMenuState::default())
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_mut::<ContextMenuState>();

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

        if !state.open {
            if let Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right)) = event {
                if cursor.is_over(layout.bounds()) {
                    if let Some(p) = cursor.position() {
                        state.open = true;
                        state.anchor = p;
                        state.reset_interaction();
                        state.ensure_focus(self.items.nodes());
                        if let Some(m) = self.on_open.clone() {
                            shell.publish(m);
                        }
                        shell.capture_event();
                        shell.request_redraw();
                    }
                }
            }
        }
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
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
        _layout: Layout<'b>,
        _renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        let state = tree.state.downcast_mut::<ContextMenuState>();
        if !state.open {
            return None;
        }

        Some(overlay::Element::new(Box::new(RootOverlay::<Message, Theme, Renderer> {
            state,
            items: &self.items,
            style: &self.style,
            submenu_mode: self.submenu_mode,
            submenu_hover_delay: self.submenu_hover_delay,
            close_on_select: self.close_on_select,
            on_close: self.on_close.clone(),
            on_select: self.on_select.as_deref(),
            viewport: *viewport,
            translation,
            _marker: PhantomData,
        })))
    }
}

struct RootOverlay<'a, 'b, Message, Theme, Renderer> {
    state: &'a mut ContextMenuState,
    items: &'b MenuSpec,
    style: &'b ContextMenuStyle,
    submenu_mode: SubmenuOpenMode,
    submenu_hover_delay: IcedDuration,
    close_on_select: bool,
    on_close: Option<Message>,
    on_select: Option<&'b dyn Fn(MenuItemId) -> Message>,
    viewport: Rectangle,
    translation: Vector,
    _marker: PhantomData<(Theme, Renderer)>,
}

impl<Message: Clone, Theme, Renderer: text::Renderer> overlay::Overlay<Message, Theme, Renderer>
    for RootOverlay<'_, '_, Message, Theme, Renderer>
{
    fn layout(&mut self, renderer: &Renderer, bounds: Size) -> layout::Node {
        let nodes = self.items.nodes();
        let (panel_node, panel_w, _panel_h) =
            layout_panel(renderer, self.style, nodes, self.state.anchor, bounds);

        self.state.submenu_anchors.clear();
        if !self.state.open_path.is_empty() {
            let ri = self.state.open_path[0];
            let pb = panel_node.bounds();
            let geoms = row_geometries(nodes, self.style);
            if let Some(g) = geoms.iter().find(|g| g.node_idx == ri) {
                let row_top = pb.y + self.style.panel_padding + self.style.border_width + g.y_offset;
                let row_bottom = row_top + g.height;
                let x = pb.x + panel_w - self.style.border_width;
                self.state
                    .submenu_anchors
                    .push(Point::new(x, row_top));
                let _ = row_bottom;
            }
        }

        let scrim = layout::Node::new(bounds);
        layout::Node::with_children(bounds, vec![scrim, panel_node])
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
        let nodes = self.items.nodes();
        self.handle_common(event, layout, cursor, shell, nodes, &[]);

        if let Event::Window(iced::window::Event::RedrawRequested(_)) = event {
            if let Some((path, started)) = self.state.submenu_delay.clone() {
                if Instant::now().duration_since(started) >= self.submenu_hover_delay {
                    let fp = self.state.focus_path.clone();
                    if fp.starts_with(&path) {
                        sync_open_path_for_focus(
                            self.state,
                            self.items,
                            SubmenuOpenMode::Hover,
                            self.submenu_hover_delay,
                            &fp,
                            shell,
                        );
                    }
                    self.state.submenu_delay = None;
                    shell.request_redraw();
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
        let _ = theme;
        let mut children = layout.children();
        let scrim_l = children.next();
        let panel_l = children.next();
        if let Some(sl) = scrim_l {
            renderer.fill_quad(
                renderer::Quad {
                    bounds: sl.bounds(),
                    ..renderer::Quad::default()
                },
                self.style.dismiss_scrim,
            );
        }
        if let Some(pl) = panel_l {
            draw_panel(
                renderer,
                self.style,
                self.items.nodes(),
                pl,
                cursor,
                &self.state.focus_path,
                &[],
                layout.bounds(),
                0,
            );
        }
    }

    fn mouse_interaction(
        &self,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _renderer: &Renderer,
    ) -> mouse::Interaction {
        if cursor.position().is_some_and(|p| layout.bounds().contains(p)) {
            mouse::Interaction::Pointer
        } else {
            mouse::Interaction::None
        }
    }

    fn overlay<'c>(
        &'c mut self,
        _layout: Layout<'c>,
        _renderer: &Renderer,
    ) -> Option<overlay::Element<'c, Message, Theme, Renderer>> {
        if self.state.open_path.is_empty() {
            return None;
        }
        let anchor = self.state.submenu_anchors.get(0).copied()?;
        Some(overlay::Element::new(Box::new(SubmenuOverlay::<Message, Theme, Renderer> {
            state: self.state,
            items: self.items,
            style: self.style,
            submenu_mode: self.submenu_mode,
            submenu_hover_delay: self.submenu_hover_delay,
            close_on_select: self.close_on_select,
            on_close: self.on_close.clone(),
            on_select: self.on_select,
            viewport: self.viewport,
            translation: self.translation,
            depth: 0,
            anchor,
            _marker: PhantomData,
        })))
    }

    fn index(&self) -> f32 {
        1.0
    }
}

impl<'a, 'b, Message: Clone, Theme, Renderer: text::Renderer> RootOverlay<'a, 'b, Message, Theme, Renderer> {
    fn handle_common(
        &mut self,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        shell: &mut Shell<'_, Message>,
        nodes: &[MenuNode],
        prefix_path: &[usize],
    ) {
        let mut children = layout.children();
        let scrim_layout = children.next();
        let panel_layout = children.next();

        match event {
            Event::Keyboard(keyboard::Event::KeyPressed {
                key: keyboard::Key::Named(keyboard::key::Named::Escape),
                ..
            }) => {
                self.state.close();
                if let Some(m) = self.on_close.clone() {
                    shell.publish(m);
                }
                shell.capture_event();
                shell.request_redraw();
                return;
            }
            _ => {}
        }

        if let (Some(sl), Some(pl)) = (scrim_layout, panel_layout) {
            if let Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) | Event::Touch(touch::Event::FingerPressed { .. }) = event
            {
                if let Some(p) = cursor.position() {
                    if sl.bounds().contains(p) && !pl.bounds().contains(p) {
                        self.state.close();
                        if let Some(m) = self.on_close.clone() {
                            shell.publish(m);
                        }
                        shell.capture_event();
                        shell.request_redraw();
                        return;
                    }
                }
            }

            if let Some(p) = cursor.position_in(pl.bounds()) {
                if let Event::Mouse(mouse::Event::CursorMoved { .. }) = event {
                    if let Some(idx) = row_index_at_y(nodes, self.style, p.y) {
                        let mut new_focus = prefix_path.to_vec();
                        new_focus.push(idx);
                        self.state.focus_path = new_focus.clone();
                        sync_open_path_for_focus(
                            self.state,
                            self.items,
                            self.submenu_mode,
                            self.submenu_hover_delay,
                            &new_focus,
                            shell,
                        );
                        shell.request_redraw();
                    }
                }

                if let Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) | Event::Touch(touch::Event::FingerPressed { .. }) = event
                {
                    if let Some(idx) = row_index_at_y(nodes, self.style, p.y) {
                        let mut path = prefix_path.to_vec();
                        path.push(idx);
                        self.activate_or_submenu_click(&path, nodes, idx, shell, true);
                    }
                }
            }
        }

        if prefix_path.is_empty() {
            self.handle_keyboard_nav(event, shell);
        }
    }

    fn activate_or_submenu_click(
        &mut self,
        path: &[usize],
        nodes: &[MenuNode],
        idx: usize,
        shell: &mut Shell<'_, Message>,
        is_root: bool,
    ) {
        let Some(node) = nodes.get(idx) else {
            return;
        };
        match node {
            MenuNode::Action { id, enabled: true, .. } => {
                if let Some(f) = self.on_select {
                    shell.publish(f(*id));
                }
                if self.close_on_select {
                    self.state.close();
                    if let Some(m) = self.on_close.clone() {
                        shell.publish(m);
                    }
                }
                shell.capture_event();
                shell.request_redraw();
            }
            MenuNode::Submenu { .. } => {
                if self.submenu_mode == SubmenuOpenMode::Click {
                    if self.state.open_path.starts_with(path) && self.state.open_path.len() == path.len() {
                        self.state.open_path.truncate(path.len().saturating_sub(1));
                    } else {
                        self.state.open_path = path.to_vec();
                    }
                    shell.capture_event();
                    shell.request_redraw();
                }
            }
            _ => {}
        }
        let _ = is_root;
    }

    fn handle_keyboard_nav(&mut self, event: &Event, shell: &mut Shell<'_, Message>) {
        let Event::Keyboard(keyboard::Event::KeyPressed { key, .. }) = event else {
            return;
        };
        let current = current_nodes(self.items.nodes(), &self.state.focus_path);
        let last_idx = *self.state.focus_path.last().unwrap_or(&0);

        match key.as_ref() {
            keyboard::Key::Named(keyboard::key::Named::ArrowDown) => {
                if let Some(i) = next_focusable(current, last_idx, 1) {
                    if self.state.focus_path.is_empty() {
                        self.state.focus_path.push(i);
                    } else {
                        *self.state.focus_path.last_mut().unwrap() = i;
                    }
                    let fp = self.state.focus_path.clone();
                    sync_open_path_for_focus(
                        self.state,
                        self.items,
                        self.submenu_mode,
                        self.submenu_hover_delay,
                        &fp,
                        shell,
                    );
                    shell.capture_event();
                    shell.request_redraw();
                }
            }
            keyboard::Key::Named(keyboard::key::Named::ArrowUp) => {
                if let Some(i) = next_focusable(current, last_idx, -1) {
                    if self.state.focus_path.is_empty() {
                        self.state.focus_path.push(i);
                    } else {
                        *self.state.focus_path.last_mut().unwrap() = i;
                    }
                    let fp = self.state.focus_path.clone();
                    sync_open_path_for_focus(
                        self.state,
                        self.items,
                        self.submenu_mode,
                        self.submenu_hover_delay,
                        &fp,
                        shell,
                    );
                    shell.capture_event();
                    shell.request_redraw();
                }
            }
            keyboard::Key::Named(keyboard::key::Named::ArrowRight) => {
                let n = node_at_path(self.items.nodes(), &self.state.focus_path);
                if let Some(MenuNode::Submenu { .. }) = n {
                    let mut p = self.state.focus_path.clone();
                    self.state.open_path = p.clone();
                    if let Some(children) = submenu_children(self.items.nodes(), &p) {
                        if let Some(ci) = first_focusable(children, None) {
                            p.push(ci);
                            self.state.focus_path = p;
                        }
                    }
                    shell.capture_event();
                    shell.request_redraw();
                }
            }
            keyboard::Key::Named(keyboard::key::Named::ArrowLeft) => {
                if self.state.focus_path.len() > 1 {
                    self.state.focus_path.pop();
                    self.state.open_path.truncate(self.state.focus_path.len().saturating_sub(1));
                    shell.capture_event();
                    shell.request_redraw();
                }
            }
            keyboard::Key::Named(keyboard::key::Named::Enter) => {
                let path = self.state.focus_path.clone();
                if let Some(&idx) = path.last() {
                    let parent = if path.len() == 1 {
                        self.items.nodes()
                    } else {
                        submenu_children(self.items.nodes(), &path[..path.len() - 1]).unwrap_or(&[])
                    };
                    self.activate_or_submenu_click(&path, parent, idx, shell, true);
                }
            }
            _ => {}
        }
    }
}

fn activate_menu_row<Message: Clone>(
    state: &mut ContextMenuState,
    submenu_mode: SubmenuOpenMode,
    close_on_select: bool,
    path: &[usize],
    nodes: &[MenuNode],
    idx: usize,
    on_close: &Option<Message>,
    on_select: Option<&dyn Fn(MenuItemId) -> Message>,
    shell: &mut Shell<'_, Message>,
) {
    let Some(node) = nodes.get(idx) else {
        return;
    };
    match node {
        MenuNode::Action { id, enabled: true, .. } => {
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

struct SubmenuOverlay<'a, 'b, Message, Theme, Renderer> {
    state: &'a mut ContextMenuState,
    items: &'b MenuSpec,
    style: &'b ContextMenuStyle,
    submenu_mode: SubmenuOpenMode,
    submenu_hover_delay: IcedDuration,
    close_on_select: bool,
    on_close: Option<Message>,
    on_select: Option<&'b dyn Fn(MenuItemId) -> Message>,
    viewport: Rectangle,
    translation: Vector,
    depth: usize,
    anchor: Point,
    _marker: PhantomData<(Theme, Renderer)>,
}

impl<Message: Clone, Theme, Renderer: text::Renderer> overlay::Overlay<Message, Theme, Renderer>
    for SubmenuOverlay<'_, '_, Message, Theme, Renderer>
{
    fn layout(&mut self, renderer: &Renderer, bounds: Size) -> layout::Node {
        if self.state.open_path.len() <= self.depth {
            return layout::Node::new(bounds);
        }
        let path = &self.state.open_path[0..=self.depth];
        let Some(nodes) = submenu_children(self.items.nodes(), path) else {
            return layout::Node::new(bounds);
        };
        let (panel_node, panel_w, _panel_h) =
            layout_panel(renderer, self.style, nodes, self.anchor, bounds);

        let next = self.depth + 1;
        if self.state.open_path.len() > next {
            let ri = self.state.open_path[next];
            let pb = panel_node.bounds();
            let geoms = row_geometries(nodes, self.style);
            if let Some(g) = geoms.iter().find(|g| g.node_idx == ri) {
                let row_top = pb.y + self.style.panel_padding + self.style.border_width + g.y_offset;
                let x = pb.x + panel_w - self.style.border_width;
                if self.state.submenu_anchors.len() <= next {
                    self.state.submenu_anchors.resize(next + 1, Point::ORIGIN);
                }
                self.state.submenu_anchors[next] = Point::new(x, row_top);
            }
        }

        layout::Node::with_children(bounds, vec![panel_node])
    }

    fn update(
        &mut self,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
    ) {
        let _ = renderer;
        if self.state.open_path.len() <= self.depth {
            return;
        }
        let path = &self.state.open_path[0..=self.depth];
        let Some(nodes) = submenu_children(self.items.nodes(), path) else {
            return;
        };
        let prefix: Vec<_> = self.state.open_path[0..=self.depth].to_vec();

        match event {
            Event::Keyboard(keyboard::Event::KeyPressed {
                key: keyboard::Key::Named(keyboard::key::Named::Escape),
                ..
            }) => {
                self.state.close();
                if let Some(m) = self.on_close.clone() {
                    shell.publish(m);
                }
                shell.capture_event();
                shell.request_redraw();
                return;
            }
            _ => {}
        }

        let panel_layout = layout.children().next();
        if let Some(pl) = panel_layout {
            if let Some(p) = cursor.position_in(pl.bounds()) {
                if let Event::Mouse(mouse::Event::CursorMoved { .. }) = event {
                    if let Some(idx) = row_index_at_y(nodes, self.style, p.y) {
                        let mut new_focus = prefix.clone();
                        new_focus.push(idx);
                        self.state.focus_path = new_focus.clone();
                        sync_open_path_for_focus(
                            self.state,
                            self.items,
                            self.submenu_mode,
                            self.submenu_hover_delay,
                            &new_focus,
                            shell,
                        );
                        shell.request_redraw();
                    }
                }

                if let Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
                | Event::Touch(touch::Event::FingerPressed { .. }) = event
                {
                    if let Some(idx) = row_index_at_y(nodes, self.style, p.y) {
                        let mut path = prefix.clone();
                        path.push(idx);
                        activate_menu_row(
                            self.state,
                            self.submenu_mode,
                            self.close_on_select,
                            &path,
                            nodes,
                            idx,
                            &self.on_close,
                            self.on_select,
                            shell,
                        );
                    }
                }
            }
        }

        if let Event::Window(iced::window::Event::RedrawRequested(_)) = event {
            if let Some((path, started)) = self.state.submenu_delay.clone() {
                if Instant::now().duration_since(started) >= self.submenu_hover_delay {
                    let fp = self.state.focus_path.clone();
                    if fp.starts_with(&path) {
                        sync_open_path_for_focus(
                            self.state,
                            self.items,
                            SubmenuOpenMode::Hover,
                            self.submenu_hover_delay,
                            &fp,
                            shell,
                        );
                    }
                    self.state.submenu_delay = None;
                    shell.request_redraw();
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
        let _ = theme;
        if self.state.open_path.len() <= self.depth {
            return;
        }
        let path = &self.state.open_path[0..=self.depth];
        let Some(nodes) = submenu_children(self.items.nodes(), path) else {
            return;
        };
        if let Some(pl) = layout.children().next() {
            draw_panel(
                renderer,
                self.style,
                nodes,
                pl,
                cursor,
                &self.state.focus_path,
                path,
                layout.bounds(),
                self.depth,
            );
        }
    }

    fn mouse_interaction(
        &self,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _renderer: &Renderer,
    ) -> mouse::Interaction {
        if let Some(pl) = layout.children().next() {
            if cursor.position().is_some_and(|p| pl.bounds().contains(p)) {
                return mouse::Interaction::Pointer;
            }
        }
        mouse::Interaction::None
    }

    fn overlay<'c>(
        &'c mut self,
        _layout: Layout<'c>,
        _renderer: &Renderer,
    ) -> Option<overlay::Element<'c, Message, Theme, Renderer>> {
        let next = self.depth + 1;
        if self.state.open_path.len() <= next {
            return None;
        }
        let anchor = self.state.submenu_anchors.get(next).copied()?;
        Some(overlay::Element::new(Box::new(SubmenuOverlay::<Message, Theme, Renderer> {
            state: self.state,
            items: self.items,
            style: self.style,
            submenu_mode: self.submenu_mode,
            submenu_hover_delay: self.submenu_hover_delay,
            close_on_select: self.close_on_select,
            on_close: self.on_close.clone(),
            on_select: self.on_select,
            viewport: self.viewport,
            translation: self.translation,
            depth: next,
            anchor,
            _marker: PhantomData,
        })))
    }

    fn index(&self) -> f32 {
        10.0 + self.depth as f32
    }
}

