//! Recursive context-menu overlay: root (`flyout_depth: None`) draws scrim + root panel;
//! nested instances (`flyout_depth: Some(d)`) match the former `SubmenuOverlay` depth `d`.

use std::marker::PhantomData;

use super::menu::{MenuItemId, MenuNode, MenuSpec, SliderStop};
use super::number::{self, FieldStyle, Fields};
use super::panel::PanelMetrics;
use super::style::{Catalog, ContextMenuStyle};

use super::panel::{
    Layout, draw_panel, icon_column, layout_panel, number_field_bounds, row_geometries,
    row_index_at_panel_y, slider_track,
};
use super::state::{
    ContextMenuState, SliderDrag, SubmenuOpenMode, current_nodes, first_focusable, next_focusable,
    node_at_path, submenu_children, sync_open_path_for_focus,
};

use iced::advanced::layout;
use iced::advanced::overlay;
use iced::advanced::renderer;
use iced::advanced::svg;
use iced::advanced::text;
use iced::advanced::widget::Operation;
use iced::advanced::{Clipboard, Shell};
use iced::keyboard;
use iced::mouse;
use iced::touch;
use iced::{Color, Element, Event, Point, Rectangle, Shadow, Size, Vector};

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
    pub(crate) class: &'a <Theme as Catalog>::Class<'b>,
    pub(crate) hotkey_label_color_override: Option<Color>,
    pub(crate) panel_shadow_override: Option<Shadow>,
    pub(crate) submenu_mode: SubmenuOpenMode,
    pub(crate) icons_enabled: bool,
    pub(crate) close_on_select: bool,
    pub(crate) on_close: Option<Message>,
    pub(crate) on_select: Option<&'a (dyn Fn(MenuItemId) -> Message + 'b)>,
    /// The number fields of this overlay's own panel, paired with their row index. Rebuilt from
    /// the spec every frame; what survives is their state, which lives in `fields`.
    pub(crate) numbers: Vec<(usize, Element<'a, Message, Theme, Renderer>)>,
    pub(crate) fields: &'a mut Fields,
    pub(crate) on_number: Option<&'a (dyn Fn(MenuItemId, f64) -> Message + 'b)>,
    pub(crate) field_style: FieldStyle,
    pub(crate) viewport: Rectangle,
    pub(crate) translation: Vector,
    pub(crate) flyout_depth: Option<usize>,
    /// Used when `flyout_depth` is `Some`; ignored for root (layout uses `state.anchor`).
    /// `x`=parent left edge, `y`=row top, `width`=parent panel width; `height` unused.
    pub(crate) anchor: Rectangle,
    pub(crate) _marker: PhantomData<(Theme, Renderer)>,
}

impl<
    'a,
    'b,
    Message: Clone + 'b,
    Theme: Catalog + iced_numbers_input::Catalog + 'b,
    Renderer: text::Renderer<Font = iced::Font> + svg::Renderer + 'b,
> MenuOverlay<'a, 'b, Message, Theme, Renderer>
where
    'b: 'a,
{
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        state: &'a mut ContextMenuState,
        items: &'a MenuSpec<'b>,
        metrics: PanelMetrics,
        class: &'a <Theme as Catalog>::Class<'b>,
        hotkey_label_color_override: Option<Color>,
        panel_shadow_override: Option<Shadow>,
        submenu_mode: SubmenuOpenMode,
        icons_enabled: bool,
        close_on_select: bool,
        on_close: Option<Message>,
        on_select: Option<&'a (dyn Fn(MenuItemId) -> Message + 'b)>,
        on_number: Option<&'a (dyn Fn(MenuItemId, f64) -> Message + 'b)>,
        field_style: FieldStyle,
        fields: &'a mut Fields,
        viewport: Rectangle,
        translation: Vector,
        flyout_depth: Option<usize>,
        anchor: Rectangle,
    ) -> Self {
        let numbers = Self::build_numbers(
            items,
            &metrics,
            &field_style,
            on_number,
            fields,
            state,
            flyout_depth,
        );

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
            numbers,
            fields,
            on_number,
            field_style,
            viewport,
            translation,
            flyout_depth,
            anchor,
            _marker: PhantomData,
        }
    }

    /// Which panel this overlay draws, as the key its fields are stored under: the root is `0` and
    /// a flyout at depth `d` is `d + 1`.
    fn panel_key(flyout_depth: Option<usize>) -> usize {
        flyout_depth.map_or(0, |depth| depth + 1)
    }

    /// The nodes of this overlay's own panel.
    fn panel_nodes(
        items: &'a MenuSpec<'b>,
        state: &ContextMenuState,
        flyout_depth: Option<usize>,
    ) -> Option<&'a [MenuNode<'a>]> {
        let Some(depth) = flyout_depth else {
            return Some(items.nodes());
        };
        if state.open_path.len() <= depth {
            return None;
        }
        submenu_children(items.nodes(), &state.open_path[0..=depth])
    }

    /// Builds this panel's number fields and reconciles each against the state it left behind last
    /// frame, so a draft being typed survives the rebuild the spec goes through every frame.
    fn build_numbers(
        items: &'a MenuSpec<'b>,
        metrics: &PanelMetrics,
        field_style: &FieldStyle,
        on_number: Option<&'a (dyn Fn(MenuItemId, f64) -> Message + 'b)>,
        fields: &mut Fields,
        state: &ContextMenuState,
        flyout_depth: Option<usize>,
    ) -> Vec<(usize, Element<'a, Message, Theme, Renderer>)> {
        let Some(on_number) = on_number else {
            return Vec::new();
        };
        let Some(nodes) = Self::panel_nodes(items, state, flyout_depth) else {
            return Vec::new();
        };
        let panel = Self::panel_key(flyout_depth);

        number::rows(nodes)
            .filter_map(|(row, node)| {
                let element = number::field(node, metrics, field_style, on_number)?;
                fields.reconcile((panel, row), &element);
                Some((row, element))
            })
            .collect()
    }

    /// The layout nodes of this panel's fields, which follow the panel's own nodes.
    fn number_layouts<'l>(
        layout: Layout<'l>,
        flyout_depth: Option<usize>,
    ) -> impl Iterator<Item = Layout<'l>> {
        // The panel's own nodes come first: a scrim and the panel for the root, the panel alone
        // for a flyout.
        let panel_nodes = match flyout_depth {
            None => 2,
            Some(_) => 1,
        };
        layout.children().skip(panel_nodes)
    }

    /// Where a panel's row sits in the viewport, the space an overlay child is laid out in.
    fn row_bounds_in_viewport(panel: &layout::Node, row: usize) -> Option<Rectangle> {
        let panel_origin = panel.bounds().position();
        let inner = panel.children().first()?;
        let inner_origin = inner.bounds().position();
        let row_node = inner.children().get(row)?;
        let bounds = row_node.bounds();

        Some(Rectangle {
            x: panel_origin.x + inner_origin.x + bounds.x,
            y: panel_origin.y + inner_origin.y + bounds.y,
            width: bounds.width,
            height: bounds.height,
        })
    }

    /// Lays this panel's fields into the room [`number_field_bounds`] leaves on each row.
    ///
    /// One node per field, in the order [`Self::numbers`] holds them, so the two can be zipped
    /// wherever a field and its layout are needed together. A row that has gone missing since the
    /// fields were built still takes its place in the list, as an empty node nothing can hit.
    fn layout_numbers(&mut self, renderer: &Renderer, panel: &layout::Node) -> Vec<layout::Node> {
        let Self {
            numbers,
            fields,
            metrics,
            flyout_depth,
            ..
        } = self;
        let key = Self::panel_key(*flyout_depth);

        numbers
            .iter_mut()
            .map(|(row, element)| {
                let placed = Self::row_bounds_in_viewport(panel, *row)
                    .zip(fields.tree_mut((key, *row)))
                    .map(|(row_bounds, tree)| {
                        let field = number_field_bounds(metrics, row_bounds);
                        let limits = layout::Limits::new(
                            Size::new(field.width, 0.0),
                            Size::new(field.width, field.height),
                        );
                        let node = element.as_widget_mut().layout(tree, renderer, &limits);
                        // The field takes the height its own content asks for, which can be less
                        // than the row leaves it, so it is centered in what it was offered.
                        let slack = (field.height - node.size().height).max(0.0);
                        node.move_to(Point::new(field.x, field.y + slack * 0.5))
                    });

                placed.unwrap_or_else(|| layout::Node::new(Size::ZERO))
            })
            .collect()
    }

    /// Hands `event` to this panel's fields. An event a field takes is left alone by the menu: a
    /// press that lands in one is an edit, not a row being picked.
    fn update_numbers(
        &mut self,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
    ) {
        let Self {
            numbers,
            fields,
            flyout_depth,
            viewport,
            ..
        } = self;
        let key = Self::panel_key(*flyout_depth);

        for ((row, element), field_layout) in numbers
            .iter_mut()
            .zip(Self::number_layouts(layout, *flyout_depth))
        {
            let Some(tree) = fields.tree_mut((key, *row)) else {
                continue;
            };
            element.as_widget_mut().update(
                tree,
                event,
                field_layout,
                cursor,
                renderer,
                clipboard,
                shell,
                viewport,
            );
        }
    }

    fn draw_numbers(
        &self,
        renderer: &mut Renderer,
        theme: &Theme,
        theme_style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
    ) {
        let key = Self::panel_key(self.flyout_depth);

        for ((row, element), field_layout) in self
            .numbers
            .iter()
            .zip(Self::number_layouts(layout, self.flyout_depth))
        {
            let Some(tree) = self.fields.tree((key, *row)) else {
                continue;
            };
            element.as_widget().draw(
                tree,
                renderer,
                theme,
                theme_style,
                field_layout,
                cursor,
                &self.viewport,
            );
        }
    }

    /// The cursor a field asks for, when the pointer is over one.
    fn numbers_mouse_interaction(
        &self,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
    ) -> Option<mouse::Interaction> {
        let key = Self::panel_key(self.flyout_depth);

        self.numbers
            .iter()
            .zip(Self::number_layouts(layout, self.flyout_depth))
            .find(|(_, field_layout)| cursor.is_over(field_layout.bounds()))
            .and_then(|((row, element), field_layout)| {
                let tree = self.fields.tree((key, *row))?;
                Some(element.as_widget().mouse_interaction(
                    tree,
                    field_layout,
                    cursor,
                    &self.viewport,
                    renderer,
                ))
            })
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
                state
                    .submenu_anchors
                    .resize(next_index + 1, Rectangle::default());
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
            // A toggle flips state in place: the menu stays open so the new checkbox is visible
            // and further rows can be picked, regardless of `close_on_select`.
            MenuNode::Toggle {
                id, enabled: true, ..
            } => {
                if let Some(f) = on_select {
                    shell.publish(f(*id));
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

    /// Path of the slider row a drag is holding, empty when no drag is live.
    fn drag_path(state: &ContextMenuState) -> &[usize] {
        state
            .slider_drag
            .as_ref()
            .map_or(&[], |drag| drag.path.as_slice())
    }

    /// Bounds of row `index` of the panel laid out at `panel_layout`.
    fn row_bounds(panel_layout: Layout<'_>, index: usize) -> Option<Rectangle> {
        panel_layout
            .children()
            .next()?
            .children()
            .nth(index)
            .map(|row| row.bounds())
    }

    fn publish_stop(
        stops: &[SliderStop<'_>],
        stop: usize,
        on_select: Option<&dyn Fn(MenuItemId) -> Message>,
        shell: &mut Shell<'_, Message>,
    ) {
        if let (Some(f), Some(stop)) = (on_select, stops.get(stop)) {
            shell.publish(f(stop.id));
        }
    }

    /// Carries on a drag that started on a slider row of this panel: the handle follows the
    /// pointer wherever it goes and reports each stop it crosses once, until the button comes up.
    #[allow(clippy::too_many_arguments)]
    fn drag_slider(
        state: &mut ContextMenuState,
        metrics: &PanelMetrics,
        icons_enabled: bool,
        on_select: Option<&dyn Fn(MenuItemId) -> Message>,
        event: &Event,
        panel_layout: Layout<'_>,
        cursor: mouse::Cursor,
        shell: &mut Shell<'_, Message>,
        nodes: &[MenuNode<'_>],
        prefix_path: &[usize],
    ) {
        let Some(drag) = state.slider_drag.clone() else {
            return;
        };
        let index = drag.path[prefix_path.len()];
        match event {
            Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                if let (Some(MenuNode::Slider { stops, .. }), Some(row), Some(position)) = (
                    nodes.get(index),
                    Self::row_bounds(panel_layout, index),
                    cursor.position(),
                ) {
                    let icon_col = icon_column(metrics, nodes, icons_enabled);
                    let track = slider_track(metrics, icon_col, row);
                    let stop = track.stop_at_x(stops.len(), position.x);
                    if stop != drag.stop {
                        state.slider_drag = Some(SliderDrag {
                            path: drag.path,
                            stop,
                        });
                        Self::publish_stop(stops, stop, on_select, shell);
                        shell.request_redraw();
                    }
                }
                shell.capture_event();
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
            | Event::Touch(touch::Event::FingerLifted { .. })
            | Event::Touch(touch::Event::FingerLost { .. }) => {
                state.slider_drag = None;
                shell.capture_event();
                shell.request_redraw();
            }
            _ => {}
        }
    }

    /// Takes the press on a slider row: the whole row belongs to the slider, so the press never
    /// reaches the dismiss layer under it, and one that lands in the track band grabs the handle.
    #[allow(clippy::too_many_arguments)]
    fn press_slider(
        state: &mut ContextMenuState,
        metrics: &PanelMetrics,
        icons_enabled: bool,
        on_select: Option<&dyn Fn(MenuItemId) -> Message>,
        panel_layout: Layout<'_>,
        cursor: mouse::Cursor,
        shell: &mut Shell<'_, Message>,
        nodes: &[MenuNode<'_>],
        path: Vec<usize>,
        index: usize,
    ) {
        shell.capture_event();
        let Some(MenuNode::Slider {
            stops,
            selected,
            enabled: true,
            ..
        }) = nodes.get(index)
        else {
            return;
        };
        if let (Some(row), Some(position)) =
            (Self::row_bounds(panel_layout, index), cursor.position())
        {
            let icon_col = icon_column(metrics, nodes, icons_enabled);
            let track = slider_track(metrics, icon_col, row);
            if track.band.contains(position) {
                let stop = track.stop_at_x(stops.len(), position.x);
                state.slider_drag = Some(SliderDrag { path, stop });
                if stop != *selected {
                    Self::publish_stop(stops, stop, on_select, shell);
                }
                shell.request_redraw();
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn update_panel_pointer(
        state: &mut ContextMenuState,
        items: &'a MenuSpec<'b>,
        metrics: &PanelMetrics,
        submenu_mode: SubmenuOpenMode,
        icons_enabled: bool,
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
        // A live slider drag owns the pointer: the panel holding it keeps the handle following,
        // and no panel re-focuses a row or opens a flyout until the drag ends.
        if let Some(drag) = state.slider_drag.as_ref() {
            if drag.path.len() == prefix_path.len() + 1 && drag.path.starts_with(prefix_path) {
                Self::drag_slider(
                    state,
                    metrics,
                    icons_enabled,
                    on_select,
                    event,
                    panel_layout,
                    cursor,
                    shell,
                    nodes,
                    prefix_path,
                );
            }
            return;
        }

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
                    if matches!(nodes.get(idx), Some(MenuNode::Slider { .. })) {
                        Self::press_slider(
                            state,
                            metrics,
                            icons_enabled,
                            on_select,
                            panel_layout,
                            cursor,
                            shell,
                            nodes,
                            path,
                            idx,
                        );
                        return;
                    }
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

    /// Whether the pointer is over the track band of a slider row of this panel.
    fn over_slider_band(
        metrics: &PanelMetrics,
        icons_enabled: bool,
        panel_layout: Layout<'_>,
        cursor: mouse::Cursor,
        nodes: &[MenuNode<'_>],
    ) -> bool {
        let Some(p) = cursor.position_in(panel_layout.bounds()) else {
            return false;
        };
        let Some(index) = row_index_at_panel_y(nodes, metrics, p.y) else {
            return false;
        };
        let Some(MenuNode::Slider { enabled: true, .. }) = nodes.get(index) else {
            return false;
        };
        let (Some(row), Some(position)) =
            (Self::row_bounds(panel_layout, index), cursor.position())
        else {
            return false;
        };
        let icon_col = icon_column(metrics, nodes, icons_enabled);
        slider_track(metrics, icon_col, row).band.contains(position)
    }

    /// Arrow keys on a focused slider row move its handle one stop instead of walking the menu.
    fn nudge_focused_slider(
        state: &ContextMenuState,
        items: &'a MenuSpec<'b>,
        direction: isize,
        on_select: Option<&dyn Fn(MenuItemId) -> Message>,
        shell: &mut Shell<'_, Message>,
    ) -> bool {
        let Some(MenuNode::Slider {
            stops,
            selected,
            enabled: true,
            ..
        }) = node_at_path(items.nodes(), &state.focus_path)
        else {
            return false;
        };
        if let Some(last) = stops.len().checked_sub(1) {
            let next = (*selected)
                .min(last)
                .saturating_add_signed(direction)
                .min(last);
            if next != *selected {
                Self::publish_stop(stops, next, on_select, shell);
            }
        }
        shell.capture_event();
        shell.request_redraw();
        true
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
                if Self::nudge_focused_slider(state, items, 1, on_select, shell) {
                    return;
                }
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
                if Self::nudge_focused_slider(state, items, -1, on_select, shell) {
                    return;
                }
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
    Message: Clone + 'b,
    Theme: Catalog + iced_numbers_input::Catalog + 'b,
    Renderer: text::Renderer<Font = iced::Font> + svg::Renderer + 'b,
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
                let mut children = vec![scrim, panel_node];
                let numbers = self.layout_numbers(renderer, &children[1]);
                children.extend(numbers);
                layout::Node::with_children(bounds, children)
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

                let mut children = vec![panel_node];
                let numbers = self.layout_numbers(renderer, &children[0]);
                children.extend(numbers);
                layout::Node::with_children(bounds, children)
            }
        }
    }

    fn update(
        &mut self,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
    ) {
        // Escape closes the menu even from inside a field: a menu the keyboard cannot dismiss is
        // worse than a draft lost.
        if let Event::Keyboard(keyboard::Event::KeyPressed {
            key: keyboard::Key::Named(keyboard::key::Named::Escape),
            ..
        }) = event
        {
            Self::handle_escape(self.state, &self.on_close, shell);
            return;
        }

        // The fields go first, and an event one of them takes ends the menu's interest in it: a
        // press inside a field is an edit, and an arrow key in a focused field steps its value
        // rather than walking the rows.
        self.update_numbers(event, layout, cursor, renderer, clipboard, shell);
        if shell.is_event_captured() {
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
                            if sl.bounds().contains(p)
                                && !pl.bounds().contains(p)
                                && self.state.slider_drag.is_none()
                            {
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
                        self.icons_enabled,
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
                        self.icons_enabled,
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
        theme_style: &renderer::Style,
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
                        Self::drag_path(self.state),
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
                        Self::drag_path(self.state),
                        layout.bounds(),
                        depth,
                        self.icons_enabled,
                    );
                }
            }
        }

        // After the panel, so a field sits on top of the row it belongs to.
        self.draw_numbers(renderer, theme, theme_style, layout, cursor);
    }

    fn mouse_interaction(
        &self,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _renderer: &Renderer,
    ) -> mouse::Interaction {
        if self.state.slider_drag.is_some() {
            return mouse::Interaction::Grabbing;
        }
        if let Some(interaction) = self.numbers_mouse_interaction(layout, cursor, _renderer) {
            return interaction;
        }
        match self.flyout_depth {
            None => {
                if Self::over_slider_band(
                    &self.metrics,
                    self.icons_enabled,
                    layout.children().nth(1).unwrap_or(layout),
                    cursor,
                    self.items.nodes(),
                ) {
                    return mouse::Interaction::Grab;
                }
                if cursor
                    .position()
                    .is_some_and(|p| layout.bounds().contains(p))
                {
                    mouse::Interaction::Pointer
                } else {
                    mouse::Interaction::None
                }
            }
            Some(depth) => {
                if self.state.open_path.len() <= depth {
                    return mouse::Interaction::None;
                }
                let path = &self.state.open_path[0..=depth];
                if let Some(pl) = layout.children().next() {
                    if cursor.position().is_some_and(|p| pl.bounds().contains(p)) {
                        if submenu_children(self.items.nodes(), path).is_some_and(|nodes| {
                            Self::over_slider_band(
                                &self.metrics,
                                self.icons_enabled,
                                pl,
                                cursor,
                                nodes,
                            )
                        }) {
                            return mouse::Interaction::Grab;
                        }
                        return mouse::Interaction::Pointer;
                    }
                }
                mouse::Interaction::None
            }
        }
    }

    /// Forwards to this panel's fields, so a focus operation reaches the text field inside one.
    fn operate(&mut self, layout: Layout<'_>, renderer: &Renderer, operation: &mut dyn Operation) {
        let Self {
            numbers,
            fields,
            flyout_depth,
            ..
        } = self;
        let key = Self::panel_key(*flyout_depth);

        for ((row, element), field_layout) in numbers
            .iter_mut()
            .zip(Self::number_layouts(layout, *flyout_depth))
        {
            let Some(tree) = fields.tree_mut((key, *row)) else {
                continue;
            };
            element
                .as_widget_mut()
                .operate(tree, field_layout, renderer, operation);
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
                    self.on_number,
                    self.field_style.clone(),
                    self.fields,
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
                    self.on_number,
                    self.field_style.clone(),
                    self.fields,
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
