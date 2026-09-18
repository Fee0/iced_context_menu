use super::menu::{MenuItemId, MenuNode, MenuSpec};
use super::open::ContextMenuOpen;
use super::panel::PanelMetrics;
use super::style::{Catalog, ContextMenuStyle, StyleFn};

use super::menu_overlay::MenuOverlay;
use super::number::{FieldStyle, Fields};
use super::panel::Layout;
use super::state::{ContextMenuState, SubmenuOpenMode};

use crate::SubmenuChevronIcon;

use iced_numbers_input::{Icon, Orientation};

use iced::advanced::layout;
use iced::advanced::overlay;
use iced::advanced::renderer;
use iced::advanced::svg;
use iced::advanced::text;
use iced::advanced::widget::Widget;
use iced::advanced::widget::tree::{self, Tree};
use iced::advanced::{Clipboard, Shell};
use iced::mouse;
use iced::{Color, Element, Event, Length, Padding, Point, Rectangle, Shadow, Size, Vector};

/// Padding inside a number field until [`ContextMenu::number_padding`] says otherwise: tight
/// vertically, so the field fits a menu row, and generous enough horizontally to hold the value
/// clear of the border.
const DEFAULT_NUMBER_PADDING: Padding = Padding {
    top: 2.0,
    right: 8.0,
    bottom: 2.0,
    left: 8.0,
};

/// What a [`ContextMenu`] keeps in the widget tree: the menu's own state, and the widget state of
/// every number field it has shown while open.
///
/// A field's state is its draft — the text being typed, which may not be a number yet — so it has
/// to outlive the frame that typed it. Nothing else the menu draws has state of its own.
#[derive(Debug, Default)]
struct TreeState {
    menu: ContextMenuState,
    fields: Fields,
}

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
    pub(crate) toggle_box_size: f32,
    pub(crate) hotkey_label_size: f32,
    pub(crate) label_hotkey_gap: f32,
    pub(crate) separator_height: f32,
    pub(crate) separator_margin_vertical: f32,
    pub(crate) row_height: f32,
    pub(crate) slider_row_height: f32,
    pub(crate) slider_track_height: f32,
    pub(crate) slider_handle_size: f32,
    pub(crate) slider_handle_halo: f32,
    pub(crate) slider_dot_size: f32,
    pub(crate) slider_min_track_width: f32,
    pub(crate) slider_label_gap: f32,
    pub(crate) number_row_height: f32,
    pub(crate) number_input_width: f32,
    pub(crate) number_text_size: f32,
    pub(crate) number_stepper_width: f32,
    pub(crate) number_padding: Padding,
    pub(crate) number_orientation: Orientation,
    pub(crate) number_icons: Option<(Icon, Icon)>,
    open: ContextMenuOpen,
    submenu_mode: SubmenuOpenMode,
    icons_enabled: bool,
    close_on_select: bool,
    on_open: Option<Message>,
    on_close: Option<Message>,
    on_select: Option<Box<dyn Fn(MenuItemId) -> Message + 'a>>,
    on_number: Option<Box<dyn Fn(MenuItemId, f64) -> Message + 'a>>,
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
            toggle_box_size: 14.0,
            hotkey_label_size: 12.0,
            label_hotkey_gap: 14.0,
            separator_height: 1.0,
            separator_margin_vertical: 6.0,
            row_height: 28.0,
            slider_row_height: 60.0,
            slider_track_height: 4.0,
            slider_handle_size: 13.0,
            slider_handle_halo: 5.0,
            slider_dot_size: 5.0,
            slider_min_track_width: 120.0,
            slider_label_gap: 8.0,
            number_row_height: 34.0,
            number_input_width: 110.0,
            number_text_size: 13.0,
            number_stepper_width: 22.0,
            number_padding: DEFAULT_NUMBER_PADDING,
            number_orientation: Orientation::default(),
            number_icons: None,
            open: ContextMenuOpen::default(),
            submenu_mode: SubmenuOpenMode::default(),
            icons_enabled: false,
            close_on_select: true,
            on_open: None,
            on_close: None,
            on_select: None,
            on_number: None,
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
            toggle_box_size: self.toggle_box_size,
            hotkey_label_size: self.hotkey_label_size,
            label_hotkey_gap: self.label_hotkey_gap,
            separator_height: self.separator_height,
            separator_margin_vertical: self.separator_margin_vertical,
            row_height: self.row_height,
            slider_row_height: self.slider_row_height,
            slider_track_height: self.slider_track_height,
            slider_handle_size: self.slider_handle_size,
            slider_handle_halo: self.slider_handle_halo,
            slider_dot_size: self.slider_dot_size,
            slider_min_track_width: self.slider_min_track_width,
            slider_label_gap: self.slider_label_gap,
            number_row_height: self.number_row_height,
            number_input_width: self.number_input_width,
            number_text_size: self.number_text_size,
            number_stepper_width: self.number_stepper_width,
            number_padding: self.number_padding,
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

    /// Height of a [`MenuNode::Slider`](crate::MenuNode::Slider) row: its label line takes
    /// [`Self::row_height`], and what is left is shared by the groove and the scale under it.
    pub fn slider_row_height(mut self, height: f32) -> Self {
        self.slider_row_height = height;
        self
    }

    /// Thickness of a slider's groove.
    pub fn slider_track_height(mut self, height: f32) -> Self {
        self.slider_track_height = height;
        self
    }

    /// Diameter of a slider's handle. It also insets the first and last stop from the groove's
    /// ends, so the handle never overhangs them.
    pub fn slider_handle_size(mut self, size: f32) -> Self {
        self.slider_handle_size = size;
        self
    }

    /// How far the handle's halo reaches past it while the pointer is on the slider. Zero draws
    /// no halo.
    pub fn slider_handle_halo(mut self, reach: f32) -> Self {
        self.slider_handle_halo = reach;
        self
    }

    /// Diameter of the dot marking each of a slider's stops, drawn on the groove itself. Zero
    /// draws no dots.
    pub fn slider_dot_size(mut self, size: f32) -> Self {
        self.slider_dot_size = size;
        self
    }

    /// Shortest groove a slider row widens the panel for.
    pub fn slider_min_track_width(mut self, width: f32) -> Self {
        self.slider_min_track_width = width;
        self
    }

    /// Smallest gap the scale under a slider leaves between two of its labels. Stops that cannot
    /// keep it go unlabelled, except the two ends, which are always spelled out.
    pub fn slider_label_gap(mut self, gap: f32) -> Self {
        self.slider_label_gap = gap;
        self
    }

    /// Height of a [`MenuNode::Number`](crate::MenuNode::Number) row, which holds a field rather
    /// than a line of text and so is taller than [`Self::row_height`].
    pub fn number_row_height(mut self, height: f32) -> Self {
        self.number_row_height = height;
        self
    }

    /// Width of the field on a [`MenuNode::Number`](crate::MenuNode::Number) row. The row's label
    /// takes what is left, and the panel grows to fit both.
    pub fn number_input_width(mut self, width: f32) -> Self {
        self.number_input_width = width;
        self
    }

    pub fn number_text_size(mut self, size: f32) -> Self {
        self.number_text_size = size;
        self
    }

    pub fn number_stepper_width(mut self, width: f32) -> Self {
        self.number_stepper_width = width;
        self
    }

    /// Padding inside a number field, around the value it shows.
    pub fn number_padding(mut self, padding: impl Into<Padding>) -> Self {
        self.number_padding = padding.into();
        self
    }

    /// How the stepper arrows of a number field are laid out.
    pub fn number_orientation(mut self, orientation: Orientation) -> Self {
        self.number_orientation = orientation;
        self
    }

    /// Glyphs the stepper arrows are drawn with. Unset leaves the field's own carets.
    pub fn number_icons(mut self, increase: impl Into<Icon>, decrease: impl Into<Icon>) -> Self {
        self.number_icons = Some((increase.into(), decrease.into()));
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

    /// Edge length of a toggle checkbox drawn in the icon slot, clamped to the row height.
    pub fn toggle_box_size(mut self, size: f32) -> Self {
        self.toggle_box_size = size;
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

    /// Called as a [`MenuNode::Number`](crate::MenuNode::Number) row's value changes — on every
    /// step, and on every keystroke that leaves the field holding a number inside its bounds.
    ///
    /// Separate from [`Self::on_select`], which carries only an id: a number row reports a value,
    /// and reports it without closing the menu so it can be dialled in.
    pub fn on_number(mut self, f: impl Fn(MenuItemId, f64) -> Message + 'a) -> Self {
        self.on_number = Some(Box::new(f));
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
    Theme: Catalog + iced_numbers_input::Catalog + 'a,
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
    Renderer: text::Renderer<Font = iced::Font> + svg::Renderer + 'a,
    Theme: Catalog + iced_numbers_input::Catalog + 'a,
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
        tree::Tag::of::<TreeState>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(TreeState::default())
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
        let state = &mut tree.state.downcast_mut::<TreeState>().menu;

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
        let TreeState { menu, fields } = tree.state.downcast_mut::<TreeState>();
        if !menu.open {
            // A draft only means anything while the row it was typed into is on screen.
            fields.clear();
            return None;
        }

        let menu = MenuOverlay::new(
            menu,
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
            self.on_number.as_deref(),
            FieldStyle {
                orientation: self.number_orientation,
                icons: self.number_icons.clone(),
            },
            fields,
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
