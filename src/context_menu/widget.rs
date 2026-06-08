use super::menu::{MenuItemId, MenuNode, MenuSpec};
use super::open::ContextMenuOpen;
use super::panel::PanelMetrics;
use super::style::{Catalog, ContextMenuStyle, StyleFn};

use super::menu_overlay::MenuOverlay;
use super::panel::Layout;
use super::state::{ContextMenuState, SubmenuOpenMode};

use crate::SubmenuChevronIcon;

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
/// Menu colors resolve from the active theme at draw time via [`Catalog`]. Pass a styling function
/// with [`.style`](Self::style), e.g. [`ContextMenuStyle::from_theme`] or [`themed`](crate::themed).
/// Spacing, sizing, and typography measurements use the builder methods (`panel_padding`, `row_height`,
/// etc.). For fixed presets, use closures such as `.style(|_| ContextMenuStyle::light())`.
///
/// ## Opening the menu
///
/// By default the menu opens on right-click over the widget. Use [`Self::opens_with`] with
/// [`ContextMenuOpen::Programmatic`] for parent-controlled open (see that variant’s docs).
pub struct ContextMenu<'a, Message, Theme = iced::Theme, Renderer = iced::Renderer>
where
    Theme: Catalog,
{
    content: Element<'a, Message, Theme, Renderer>,
    pub(crate) items: MenuSpec<'a>,
    class: Theme::Class<'a>,
    hotkey_label_color_override: Option<Color>,
    panel_shadow_override: Option<Shadow>,
    pub(crate) border_width: f32,
    pub(crate) border_radius: f32,
    pub(crate) panel_padding: f32,
    pub(crate) row_label_inset: f32,
    pub(crate) min_width: f32,
    pub(crate) row_spacing: f32,
    pub(crate) label_size: f32,
    pub(crate) submenu_chevron_icon: SubmenuChevronIcon,
    pub(crate) submenu_chevron_slot_width: f32,
    pub(crate) submenu_flyout_overlap: f32,
    pub(crate) icon_slot_width: f32,
    pub(crate) icon_label_gap: f32,
    pub(crate) icon_glyph_size: f32,
    pub(crate) hotkey_label_size: f32,
    pub(crate) label_hotkey_gap: f32,
    pub(crate) separator_height: f32,
    pub(crate) separator_margin_vertical: f32,
    pub(crate) row_height: f32,
    open: ContextMenuOpen,
    submenu_mode: SubmenuOpenMode,
    icons_enabled: bool,
    close_on_select: bool,
    on_open: Option<Message>,
    on_close: Option<Message>,
    on_select: Option<Box<dyn Fn(MenuItemId) -> Message + 'a>>,
}

impl<'a, Message, Theme, Renderer> ContextMenu<'a, Message, Theme, Renderer>
where
    Theme: Catalog,
{
    /// Builds a menu with theme-derived default styling and hover submenus.
    pub fn new(content: impl Into<Element<'a, Message, Theme, Renderer>>) -> Self {
        Self {
            content: content.into(),
            items: MenuSpec::default(),
            class: Theme::default(),
            hotkey_label_color_override: None,
            panel_shadow_override: None,
            border_width: 1.0,
            border_radius: 6.0,
            panel_padding: 6.0,
            row_label_inset: 6.0,
            min_width: 160.0,
            row_spacing: 2.0,
            label_size: 14.0,
            submenu_chevron_icon: SubmenuChevronIcon::default(),
            submenu_chevron_slot_width: 20.0,
            submenu_flyout_overlap: 5.0,
            icon_slot_width: 18.0,
            icon_label_gap: 6.0,
            icon_glyph_size: 16.0,
            hotkey_label_size: 12.0,
            label_hotkey_gap: 14.0,
            separator_height: 1.0,
            separator_margin_vertical: 6.0,
            row_height: 28.0,
            open: ContextMenuOpen::default(),
            submenu_mode: SubmenuOpenMode::default(),
            icons_enabled: false,
            close_on_select: true,
            on_open: None,
            on_close: None,
            on_select: None,
        }
    }

    pub(crate) fn panel_metrics(&self) -> PanelMetrics {
        PanelMetrics {
            border_width: self.border_width,
            border_radius: self.border_radius,
            panel_padding: self.panel_padding,
            row_label_inset: self.row_label_inset,
            min_width: self.min_width,
            row_spacing: self.row_spacing,
            label_size: self.label_size,
            submenu_chevron_icon: self.submenu_chevron_icon,
            submenu_chevron_slot_width: self.submenu_chevron_slot_width,
            submenu_flyout_overlap: self.submenu_flyout_overlap,
            icon_slot_width: self.icon_slot_width,
            icon_label_gap: self.icon_label_gap,
            icon_glyph_size: self.icon_glyph_size,
            hotkey_label_size: self.hotkey_label_size,
            label_hotkey_gap: self.label_hotkey_gap,
            separator_height: self.separator_height,
            separator_margin_vertical: self.separator_margin_vertical,
            row_height: self.row_height,
        }
    }

    pub fn items(mut self, spec: MenuSpec<'a>) -> Self {
        self.items = spec;
        self
    }

    /// Sets the styling function for menu colors and effects; layout fields are unchanged.
    #[must_use]
    pub fn style(mut self, style: impl Fn(&Theme) -> ContextMenuStyle + 'a) -> Self
    where
        Theme::Class<'a>: From<StyleFn<'a, Theme>>,
    {
        self.class = (Box::new(style) as StyleFn<'a, Theme>).into();
        self
    }

    pub fn panel_padding(mut self, padding: f32) -> Self {
        self.panel_padding = padding;
        self
    }

    pub fn min_width(mut self, width: f32) -> Self {
        self.min_width = width;
        self
    }

    pub fn label_size(mut self, size: f32) -> Self {
        self.label_size = size;
        self
    }

    pub fn row_height(mut self, height: f32) -> Self {
        self.row_height = height;
        self
    }

    pub fn row_spacing(mut self, spacing: f32) -> Self {
        self.row_spacing = spacing;
        self
    }

    pub fn border_radius(mut self, radius: f32) -> Self {
        self.border_radius = radius;
        self
    }

    pub fn border_width(mut self, width: f32) -> Self {
        self.border_width = width;
        self
    }

    pub fn row_label_inset(mut self, inset: f32) -> Self {
        self.row_label_inset = inset;
        self
    }

    pub fn submenu_flyout_overlap(mut self, overlap: f32) -> Self {
        self.submenu_flyout_overlap = overlap;
        self
    }

    pub fn submenu_chevron_icon(mut self, icon: SubmenuChevronIcon) -> Self {
        self.submenu_chevron_icon = icon;
        self
    }

    pub fn submenu_chevron_slot_width(mut self, width: f32) -> Self {
        self.submenu_chevron_slot_width = width;
        self
    }

    pub fn hotkey_label_size(mut self, size: f32) -> Self {
        self.hotkey_label_size = size;
        self
    }

    pub fn label_hotkey_gap(mut self, gap: f32) -> Self {
        self.label_hotkey_gap = gap;
        self
    }

    pub fn hotkey_label_color(mut self, color: Color) -> Self {
        self.hotkey_label_color_override = Some(color);
        self
    }

    pub fn icon_slot_width(mut self, width: f32) -> Self {
        self.icon_slot_width = width;
        self
    }

    pub fn icon_label_gap(mut self, gap: f32) -> Self {
        self.icon_label_gap = gap;
        self
    }

    pub fn icon_glyph_size(mut self, size: f32) -> Self {
        self.icon_glyph_size = size;
        self
    }

    pub fn separator_height(mut self, height: f32) -> Self {
        self.separator_height = height;
        self
    }

    pub fn separator_margin_vertical(mut self, margin: f32) -> Self {
        self.separator_margin_vertical = margin;
        self
    }

    pub fn panel_shadow(mut self, shadow: Shadow) -> Self {
        self.panel_shadow_override = Some(shadow);
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

pub(crate) fn resolve_menu_style<Theme: Catalog>(
    theme: &Theme,
    class: &Theme::Class<'_>,
    hotkey_label_color_override: Option<Color>,
    panel_shadow_override: Option<Shadow>,
) -> ContextMenuStyle {
    let mut style = theme.style(class);
    if let Some(c) = hotkey_label_color_override {
        style.hotkey_label_color = c;
    }
    if let Some(s) = panel_shadow_override {
        style.panel_shadow = s;
    }
    style
}

impl<'a, Message, Theme, Renderer> From<ContextMenu<'a, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: Clone + 'a,
    Theme: Catalog + 'a,
    Renderer: 'a + text::Renderer<Font = iced::Font> + svg::Renderer,
{
    fn from(menu: ContextMenu<'a, Message, Theme, Renderer>) -> Self {
        Element::new(menu)
    }
}

impl<'a, Message: Clone, Theme, Renderer> Widget<Message, Theme, Renderer>
    for ContextMenu<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Renderer: text::Renderer<Font = iced::Font> + svg::Renderer,
    Theme: Catalog + 'a,
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

        let menu = MenuOverlay::new(
            state,
            &self.items,
            self.panel_metrics(),
            &self.class,
            self.hotkey_label_color_override,
            self.panel_shadow_override,
            self.submenu_mode,
            self.icons_enabled,
            self.close_on_select,
            self.on_close.clone(),
            self.on_select.as_deref(),
            *viewport,
            translation,
            None,
            Rectangle::default(),
        );

        Some(overlay::Element::new(Box::new(menu)))
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
