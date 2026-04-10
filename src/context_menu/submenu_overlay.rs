use std::marker::PhantomData;

use super::menu::{MenuItemId, MenuNode, MenuSpec};
use super::style::ContextMenuStyle;

use super::panel::{draw_panel, layout_panel, row_geometries, row_index_at_panel_y, Layout};
use super::state::{sync_open_path_for_focus, ContextMenuState, SubmenuOpenMode};

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

pub(crate) struct SubmenuOverlay<'a, 'b, Message, Theme, Renderer> {
    pub(crate) state: &'a mut ContextMenuState,
    pub(crate) items: &'b MenuSpec,
    pub(crate) style: &'b ContextMenuStyle,
    pub(crate) submenu_mode: SubmenuOpenMode,
    pub(crate) close_on_select: bool,
    pub(crate) on_close: Option<Message>,
    pub(crate) on_select: Option<&'b dyn Fn(MenuItemId) -> Message>,
    pub(crate) viewport: Rectangle,
    pub(crate) translation: Vector,
    pub(crate) depth: usize,
    pub(crate) anchor: Point,
    pub(crate) _marker: PhantomData<(Theme, Renderer)>,
}

impl<Message: Clone, Theme, Renderer: text::Renderer + svg::Renderer>
    overlay::Overlay<Message, Theme, Renderer> for SubmenuOverlay<'_, '_, Message, Theme, Renderer>
{
    fn layout(&mut self, renderer: &Renderer, bounds: Size) -> layout::Node {
        if self.state.open_path.len() <= self.depth {
            return layout::Node::new(bounds);
        }
        let path = &self.state.open_path[0..=self.depth];
        let Some(nodes) = super::state::submenu_children(self.items.nodes(), path) else {
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
        let Some(nodes) = super::state::submenu_children(self.items.nodes(), path) else {
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
                    if let Some(idx) = row_index_at_panel_y(nodes, self.style, p.y) {
                        let mut new_focus = prefix.clone();
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
        let Some(nodes) = super::state::submenu_children(self.items.nodes(), path) else {
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
