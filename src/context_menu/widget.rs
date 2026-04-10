use std::marker::PhantomData;

use super::menu::{MenuItemId, MenuNode, MenuSpec};
use super::open::ContextMenuOpen;
use super::style::ContextMenuStyle;

use super::menu_overlay::MenuOverlay;
use super::panel::Layout;
use super::state::{ContextMenuState, SubmenuOpenMode};

use iced::advanced::layout;
use iced::advanced::overlay;
use iced::advanced::renderer;
use iced::advanced::svg;
use iced::advanced::text;
use iced::advanced::widget::Widget;
use iced::advanced::widget::tree::{self, Tree};
use iced::advanced::{Clipboard, Shell};
use iced::mouse;
use iced::{Color, Element, Event, Length, Point, Rectangle, Shadow, Size, Vector};

/// Right-click wrapper that shows a [`MenuSpec`](super::menu::MenuSpec) in an overlay. The menu
/// shares the widget lifetime `'a` with the inner [`Element`](iced::Element) so row text can borrow
/// from application state.
///
/// ## Theming
///
/// Pass a full [`ContextMenuStyle`] with [`Self::style`], or override common fields with the
/// builder methods (`panel_padding`, `row_label_inset`, [`Self::panel_shadow`], etc.). For any
/// field without a dedicated method, use `let mut s = ContextMenuStyle::example_dark(); s.separator_color = …;`
/// then `.style(s)`.
///
/// ## Opening the menu
///
/// By default the menu opens on right-click over the widget. Use [`Self::opens_with`] with
/// [`ContextMenuOpen::Programmatic`] for parent-controlled open (see that variant’s docs).
pub struct ContextMenu<'a, Message, Theme = iced::Theme, Renderer = iced::Renderer> {
    content: Element<'a, Message, Theme, Renderer>,
    items: MenuSpec<'a>,
    style: ContextMenuStyle,
    open: ContextMenuOpen,
    submenu_mode: SubmenuOpenMode,
    icons_enabled: bool,
    close_on_select: bool,
    on_open: Option<Message>,
    on_close: Option<Message>,
    on_select: Option<Box<dyn Fn(MenuItemId) -> Message + 'a>>,
}

impl<'a, Message, Theme, Renderer> ContextMenu<'a, Message, Theme, Renderer> {
    /// Builds a menu with [`ContextMenuStyle::default`] and hover submenus.
    pub fn new(content: impl Into<Element<'a, Message, Theme, Renderer>>) -> Self {
        Self {
            content: content.into(),
            items: MenuSpec::default(),
            style: ContextMenuStyle::default(),
            open: ContextMenuOpen::default(),
            submenu_mode: SubmenuOpenMode::default(),
            icons_enabled: false,
            close_on_select: true,
            on_open: None,
            on_close: None,
            on_select: None,
        }
    }

    pub fn items(mut self, spec: MenuSpec<'a>) -> Self {
        self.items = spec;
        self
    }

    /// Replaces the entire style. Combine with builder methods by calling this first, or mutate
    /// a [`ContextMenuStyle`] value before passing it here.
    pub fn style(mut self, style: ContextMenuStyle) -> Self {
        self.style = style;
        self
    }

    pub fn panel_padding(mut self, padding: f32) -> Self {
        self.style.panel_padding = padding;
        self
    }

    pub fn min_width(mut self, width: f32) -> Self {
        self.style.min_width = width;
        self
    }

    pub fn label_size(mut self, size: f32) -> Self {
        self.style.label_size = size;
        self
    }

    pub fn row_height(mut self, height: f32) -> Self {
        self.style.row_height = height;
        self
    }

    pub fn row_spacing(mut self, spacing: f32) -> Self {
        self.style.row_spacing = spacing;
        self
    }

    pub fn border_radius(mut self, radius: f32) -> Self {
        self.style.border_radius = radius;
        self
    }

    pub fn border_width(mut self, width: f32) -> Self {
        self.style.border_width = width;
        self
    }

    pub fn row_label_inset(mut self, inset: f32) -> Self {
        self.style.row_label_inset = inset;
        self
    }

    pub fn submenu_flyout_overlap(mut self, overlap: f32) -> Self {
        self.style.submenu_flyout_overlap = overlap;
        self
    }

    pub fn hotkey_label_size(mut self, size: f32) -> Self {
        self.style.hotkey_label_size = size;
        self
    }

    pub fn label_hotkey_gap(mut self, gap: f32) -> Self {
        self.style.label_hotkey_gap = gap;
        self
    }

    pub fn hotkey_label_color(mut self, color: Color) -> Self {
        self.style.hotkey_label_color = color;
        self
    }

    pub fn icon_slot_width(mut self, width: f32) -> Self {
        self.style.icon_slot_width = width;
        self
    }

    pub fn icon_label_gap(mut self, gap: f32) -> Self {
        self.style.icon_label_gap = gap;
        self
    }

    pub fn panel_shadow(mut self, shadow: Shadow) -> Self {
        self.style.panel_shadow = shadow;
        self
    }

    pub fn submenu_open_mode(mut self, mode: SubmenuOpenMode) -> Self {
        self.submenu_mode = mode;
        self
    }

    pub fn opens_with(mut self, mode: ContextMenuOpen) -> Self {
        self.open = mode;
        self
    }

    pub fn show_item_icons(mut self, show: bool) -> Self {
        self.icons_enabled = show;
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
    Renderer: 'a + text::Renderer + svg::Renderer,
{
    fn from(menu: ContextMenu<'a, Message, Theme, Renderer>) -> Self {
        Element::new(menu)
    }
}

impl<'a, Message: Clone, Theme, Renderer> Widget<Message, Theme, Renderer>
    for ContextMenu<'a, Message, Theme, Renderer>
where
    Renderer: text::Renderer + svg::Renderer,
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
            let nodes = self.items.nodes();
            match self.open {
                ContextMenuOpen::RightClick => {
                    if let Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right)) = event {
                        if cursor.is_over(layout.bounds()) {
                            if let Some(p) = cursor.position() {
                                open_menu_at(state, nodes, p, &self.on_open, shell);
                            }
                        }
                    }
                }
                ContextMenuOpen::Programmatic { open, anchor } => {
                    if open {
                        let bounds = layout.bounds();
                        let p = match anchor {
                            Some(p) => p,
                            None => cursor
                                .position()
                                .filter(|_| cursor.is_over(bounds))
                                .unwrap_or_else(|| bounds_center(bounds)),
                        };
                        open_menu_at(state, nodes, p, &self.on_open, shell);
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

        Some(overlay::Element::new(Box::new(MenuOverlay::<
            Message,
            Theme,
            Renderer,
        > {
            state,
            items: &self.items,
            style: &self.style,
            submenu_mode: self.submenu_mode,
            icons_enabled: self.icons_enabled,
            close_on_select: self.close_on_select,
            on_close: self.on_close.clone(),
            on_select: self.on_select.as_deref(),
            viewport: *viewport,
            translation,
            flyout_depth: None,
            anchor: Point::ORIGIN,
            _marker: PhantomData,
        })))
    }
}

fn bounds_center(bounds: Rectangle) -> Point {
    Point::new(
        bounds.x + bounds.width * 0.5,
        bounds.y + bounds.height * 0.5,
    )
}

fn open_menu_at<Message: Clone>(
    state: &mut ContextMenuState,
    nodes: &[MenuNode<'_>],
    anchor: Point,
    on_open: &Option<Message>,
    shell: &mut Shell<'_, Message>,
) {
    state.open = true;
    state.anchor = anchor;
    state.reset_interaction();
    state.ensure_focus(nodes);
    if let Some(m) = on_open.clone() {
        shell.publish(m);
    }
    shell.capture_event();
    shell.request_redraw();
}
