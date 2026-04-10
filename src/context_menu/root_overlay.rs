use std::marker::PhantomData;

use super::menu::{MenuItemId, MenuNode, MenuSpec};
use super::style::ContextMenuStyle;

use super::panel::{draw_panel, layout_panel, row_geometries, row_index_at_panel_y, Layout};
use super::state::{
    current_nodes, first_focusable, next_focusable, node_at_path, submenu_children,
    sync_open_path_for_focus, ContextMenuState, SubmenuOpenMode,
};
use super::submenu_overlay::SubmenuOverlay;

use iced::advanced::layout;
use iced::advanced::overlay;
use iced::advanced::renderer;
use iced::advanced::svg;
use iced::advanced::text;
use iced::advanced::{Clipboard, Shell};
use iced::keyboard;
use iced::mouse;
use iced::touch;
use iced::{Event, Point, Rectangle, Size, Vector};

pub(crate) struct RootOverlay<'a, 'b, Message, Theme, Renderer> {
    pub(crate) state: &'a mut ContextMenuState,
    pub(crate) items: &'b MenuSpec,
    pub(crate) style: &'b ContextMenuStyle,
    pub(crate) submenu_mode: SubmenuOpenMode,
    pub(crate) icons_enabled: bool,
    pub(crate) close_on_select: bool,
    pub(crate) on_close: Option<Message>,
    pub(crate) on_select: Option<&'b dyn Fn(MenuItemId) -> Message>,
    pub(crate) viewport: Rectangle,
    pub(crate) translation: Vector,
    pub(crate) _marker: PhantomData<(Theme, Renderer)>,
}

impl<Message: Clone, Theme, Renderer: text::Renderer + svg::Renderer>
    overlay::Overlay<Message, Theme, Renderer> for RootOverlay<'_, '_, Message, Theme, Renderer>
{
    fn layout(&mut self, renderer: &Renderer, bounds: Size) -> layout::Node {
        let nodes = self.items.nodes();
        let (panel_node, panel_w, _panel_h) = layout_panel(
            renderer,
            self.style,
            nodes,
            self.state.anchor,
            bounds,
            self.icons_enabled,
        );

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
                &self.state.open_path,
                layout.bounds(),
                0,
                self.icons_enabled,
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
            icons_enabled: self.icons_enabled,
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

impl<'a, 'b, Message: Clone, Theme, Renderer: text::Renderer + svg::Renderer>
    RootOverlay<'a, 'b, Message, Theme, Renderer>
{
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
            if let Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
            | Event::Touch(touch::Event::FingerPressed { .. }) = event
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
                    if let Some(idx) = row_index_at_panel_y(nodes, self.style, p.y) {
                        let mut new_focus = prefix_path.to_vec();
                        new_focus.push(idx);
                        self.state.focus_path = new_focus.clone();
                        sync_open_path_for_focus(
                            self.state,
                            self.items,
                            self.submenu_mode,
                            &new_focus,
                        );
                        shell.request_redraw();
                    }
                }

                if let Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
                | Event::Touch(touch::Event::FingerPressed { .. }) = event
                {
                    if let Some(idx) = row_index_at_panel_y(nodes, self.style, p.y) {
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
                    if self.state.open_path.starts_with(path) && self.state.open_path.len() == path.len()
                    {
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
                    sync_open_path_for_focus(self.state, self.items, self.submenu_mode, &fp);
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
                    sync_open_path_for_focus(self.state, self.items, self.submenu_mode, &fp);
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
                    self.state
                        .open_path
                        .truncate(self.state.focus_path.len().saturating_sub(1));
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
