use iced::widget::{
    button, checkbox, column, container, radio, row, rule, scrollable, slider, text,
};
use iced::window::Settings;
use iced::{Color, Element, Length, Size, Task};
use iced_context_menu::{
    ContextMenu, ContextMenuOpen, ContextMenuStyle, MenuIcon, MenuItemId, MenuSpec, Shaping,
    SubmenuChevronIcon, SubmenuOpenMode,
};

fn main() -> iced::Result {
    iced::application(|| State::default(), update, view)
        .window(Settings {
            size: Size::new(1500.0, 1000.0),
            ..Settings::default()
        })
        .run()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StylePreset {
    Dark,
    Light,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum DemoOpenMode {
    #[default]
    RightClick,
    Programmatic,
}

fn merged_style(state: &State) -> impl Fn(&iced::Theme) -> ContextMenuStyle + '_ {
    move |_theme| {
        let mut s = match state.style_preset {
            StylePreset::Dark => ContextMenuStyle::dark(),
            StylePreset::Light => ContextMenuStyle::light(),
        };
        s.panel_shadow.blur_radius = state.panel_shadow_blur;
        s.dismiss_scrim = Color::from_rgba(0.0, 0.0, 0.0, state.scrim_alpha);
        let hk = s.hotkey_label_color;
        s.hotkey_label_color = Color::from_rgba(hk.r, hk.g, hk.b, hk.a * state.hotkey_label_alpha);
        s
    }
}

#[derive(Debug, Clone)]
enum Message {
    MenuOpened,
    MenuClosed,
    MenuSelected(MenuItemId),
    SubmenuMode(SubmenuOpenMode),
    ShowItemIcons(bool),
    CloseOnSelect(bool),
    PanelPadding(f32),
    MinWidth(f32),
    LabelSize(f32),
    RowHeight(f32),
    RowSpacing(f32),
    BorderRadius(f32),
    BorderWidth(f32),
    SubmenuFlyoutOverlap(f32),
    PanelShadowBlur(f32),
    ScrimAlpha(f32),
    HotkeyLabelAlpha(f32),
    RowLabelInset(f32),
    SubmenuChevronIcon(SubmenuChevronIcon),
    SubmenuChevronSlotWidth(f32),
    IconSlotWidth(f32),
    IconLabelGap(f32),
    IconGlyphSize(f32),
    HotkeyLabelSize(f32),
    LabelHotkeyGap(f32),
    SeparatorHeight(f32),
    SeparatorMarginVertical(f32),
    StylePreset(StylePreset),
    DemoOpenMode(DemoOpenMode),
    RequestProgrammaticOpen,
}

#[derive(Debug, Clone)]
struct State {
    status: String,
    submenu_mode: SubmenuOpenMode,
    show_item_icons: bool,
    close_on_select: bool,
    panel_padding: f32,
    min_width: f32,
    label_size: f32,
    row_height: f32,
    row_spacing: f32,
    border_radius: f32,
    border_width: f32,
    submenu_flyout_overlap: f32,
    panel_shadow_blur: f32,
    scrim_alpha: f32,
    hotkey_label_alpha: f32,
    row_label_inset: f32,
    submenu_chevron_icon: SubmenuChevronIcon,
    submenu_chevron_slot_width: f32,
    icon_slot_width: f32,
    icon_label_gap: f32,
    icon_glyph_size: f32,
    hotkey_label_size: f32,
    label_hotkey_gap: f32,
    separator_height: f32,
    separator_margin_vertical: f32,
    style_preset: StylePreset,
    demo_open_mode: DemoOpenMode,
    /// One-shot: set from the button, cleared when the menu opens.
    programmatic_open_pulse: bool,
}

impl Default for State {
    fn default() -> Self {
        Self {
            status: String::new(),
            submenu_mode: SubmenuOpenMode::Hover,
            show_item_icons: true,
            close_on_select: true,
            panel_padding: 6.0,
            min_width: 160.0,
            label_size: 14.0,
            row_height: 28.0,
            row_spacing: 2.0,
            border_radius: 6.0,
            border_width: 1.0,
            submenu_flyout_overlap: 5.0,
            panel_shadow_blur: 12.0,
            scrim_alpha: 0.15,
            hotkey_label_alpha: 1.0,
            row_label_inset: 6.0,
            submenu_chevron_icon: SubmenuChevronIcon::default(),
            submenu_chevron_slot_width: 20.0,
            icon_slot_width: 18.0,
            icon_label_gap: 6.0,
            icon_glyph_size: 16.0,
            hotkey_label_size: 12.0,
            label_hotkey_gap: 14.0,
            separator_height: 1.0,
            separator_margin_vertical: 6.0,
            style_preset: StylePreset::Dark,
            demo_open_mode: DemoOpenMode::default(),
            programmatic_open_pulse: false,
        }
    }
}

fn demo_row_icon() -> MenuIcon {
    MenuIcon::from_svg_bytes(include_bytes!("../svg/copy-svgrepo-com.svg"))
}

fn demo_row_icon2() -> MenuIcon {
    MenuIcon::from_svg_bytes(include_bytes!("../svg/paste-svgrepo-com.svg"))
}

fn demo_glyph_icon() -> MenuIcon {
    MenuIcon::from_glyph("\u{2605}", None, Shaping::Advanced)
}

fn build_menu() -> MenuSpec<'static> {
    let more_children = MenuSpec::new()
        .action(4_u64, "Rename", None, None)
        .submenu(
            "Share",
            MenuSpec::new()
                .action(5_u64, "Copy link", None, None)
                .action(6_u64, "Open permissions", None, None)
                .nodes()
                .to_vec(),
            None,
        )
        .nodes()
        .to_vec();

    let more_with_icon_children = MenuSpec::new()
        .action(7_u64, "Rename", None, Some("F10".into()))
        .submenu(
            "Share",
            MenuSpec::new()
                .action(8_u64, "Copy link", None, Some("F11".into()))
                .action(9_u64, "Open permissions", None, None)
                .nodes()
                .to_vec(),
            None,
        )
        .nodes()
        .to_vec();

    MenuSpec::new()
        .action(
            1_u64,
            "Copy".to_string(),
            Some(demo_row_icon()),
            Some("Ctrl+C".into()),
        )
        .action(
            2_u64,
            "Paste",
            Some(demo_glyph_icon()),
            Some("Ctrl+V".into()),
        )
        .separator()
        .disabled(3_u64, "Unavailable", None, None)
        .submenu("More", more_children, None)
        .submenu("More", more_with_icon_children, Some(demo_row_icon2()))
}

fn update(state: &mut State, message: Message) -> Task<Message> {
    match message {
        Message::MenuOpened => {
            state.status = "Menu opened".to_string();
            state.programmatic_open_pulse = false;
        }
        Message::MenuClosed => state.status = "Menu closed".to_string(),
        Message::MenuSelected(id) => {
            state.status = format!("Selected item {}", id);
        }
        Message::SubmenuMode(m) => state.submenu_mode = m,
        Message::ShowItemIcons(v) => state.show_item_icons = v,
        Message::CloseOnSelect(v) => state.close_on_select = v,
        Message::PanelPadding(v) => state.panel_padding = v,
        Message::MinWidth(v) => state.min_width = v,
        Message::LabelSize(v) => state.label_size = v,
        Message::RowHeight(v) => state.row_height = v,
        Message::RowSpacing(v) => state.row_spacing = v,
        Message::BorderRadius(v) => state.border_radius = v,
        Message::BorderWidth(v) => state.border_width = v,
        Message::SubmenuFlyoutOverlap(v) => state.submenu_flyout_overlap = v,
        Message::PanelShadowBlur(v) => state.panel_shadow_blur = v,
        Message::ScrimAlpha(v) => state.scrim_alpha = v,
        Message::HotkeyLabelAlpha(v) => state.hotkey_label_alpha = v,
        Message::RowLabelInset(v) => state.row_label_inset = v,
        Message::SubmenuChevronIcon(i) => state.submenu_chevron_icon = i,
        Message::SubmenuChevronSlotWidth(v) => state.submenu_chevron_slot_width = v,
        Message::IconSlotWidth(v) => state.icon_slot_width = v,
        Message::IconLabelGap(v) => state.icon_label_gap = v,
        Message::IconGlyphSize(v) => state.icon_glyph_size = v,
        Message::HotkeyLabelSize(v) => state.hotkey_label_size = v,
        Message::LabelHotkeyGap(v) => state.label_hotkey_gap = v,
        Message::SeparatorHeight(v) => state.separator_height = v,
        Message::SeparatorMarginVertical(v) => state.separator_margin_vertical = v,
        Message::StylePreset(p) => state.style_preset = p,
        Message::DemoOpenMode(m) => {
            state.demo_open_mode = m;
            state.programmatic_open_pulse = false;
        }
        Message::RequestProgrammaticOpen => state.programmatic_open_pulse = true,
    }

    Task::none()
}

fn labeled_slider<'a>(
    label: &'static str,
    range: std::ops::RangeInclusive<f32>,
    value: f32,
    fmt: impl Fn(f32) -> String + 'a,
    on_change: impl Fn(f32) -> Message + Clone + 'a,
    step: Option<f32>,
) -> Element<'a, Message> {
    let row_label = text(format!("{}: {}", label, fmt(value)));
    let sl = slider(range, value, on_change);
    let sl = match step {
        Some(st) => sl.step(st),
        None => sl,
    }
    .width(Length::Fill);
    column![row_label, sl].spacing(4).into()
}

fn view(state: &State) -> Element<'_, Message> {
    let behavior = column![
        text("How nested submenus open:"),
        column![
            radio(
                "Hover",
                SubmenuOpenMode::Hover,
                Some(state.submenu_mode),
                Message::SubmenuMode,
            ),
            radio(
                "Click",
                SubmenuOpenMode::Click,
                Some(state.submenu_mode),
                Message::SubmenuMode,
            ),
        ]
        .spacing(4),
        rule::horizontal(1),
        checkbox(state.show_item_icons)
            .label("Show icons")
            .on_toggle(Message::ShowItemIcons),
        checkbox(state.close_on_select)
            .label("Close menu after selecting an action")
            .on_toggle(Message::CloseOnSelect),
        rule::horizontal(1),
        text("How the root menu opens:"),
        column![
            radio(
                "Right-click on target",
                DemoOpenMode::RightClick,
                Some(state.demo_open_mode),
                Message::DemoOpenMode,
            ),
            radio(
                "Programmatic",
                DemoOpenMode::Programmatic,
                Some(state.demo_open_mode),
                Message::DemoOpenMode,
            ),
        ]
        .spacing(4),
    ]
    .spacing(8);

    let theme = column![
        text("Theme preset").size(14),
        column![
            radio(
                "Dark",
                StylePreset::Dark,
                Some(state.style_preset),
                Message::StylePreset,
            ),
            radio(
                "Light",
                StylePreset::Light,
                Some(state.style_preset),
                Message::StylePreset,
            ),
        ]
        .spacing(4),
        labeled_slider(
            "Panel shadow blur",
            0.0..=100.0,
            state.panel_shadow_blur,
            |x| format!("{:.0}px", x),
            Message::PanelShadowBlur,
            None,
        ),
        labeled_slider(
            "Dismiss scrim opacity",
            0.0..=1.0,
            state.scrim_alpha,
            |x| format!("{:.2}", x),
            Message::ScrimAlpha,
            Some(0.01),
        ),
        labeled_slider(
            "Hotkey hint opacity",
            0.0..=1.0,
            state.hotkey_label_alpha,
            |x| format!("{:.2}", x),
            Message::HotkeyLabelAlpha,
            Some(0.01),
        ),
    ]
    .spacing(8);

    let panel = column![
        text("Panel").size(14),
        labeled_slider(
            "Panel padding",
            1.0..=40.0,
            state.panel_padding,
            |x| format!("{:.0}px", x),
            Message::PanelPadding,
            None,
        ),
        labeled_slider(
            "Min width",
            120.0..=400.0,
            state.min_width,
            |x| format!("{:.0}px", x),
            Message::MinWidth,
            None,
        ),
        labeled_slider(
            "Row label inset",
            0.0..=30.0,
            state.row_label_inset,
            |x| format!("{:.0}px", x),
            Message::RowLabelInset,
            None,
        ),
        labeled_slider(
            "Border radius",
            0.0..=40.0,
            state.border_radius,
            |x| format!("{:.0}px", x),
            Message::BorderRadius,
            None,
        ),
        labeled_slider(
            "Border width",
            0.0..=20.0,
            state.border_width,
            |x| format!("{:.1}px", x),
            Message::BorderWidth,
            None,
        ),
        labeled_slider(
            "Separator height",
            0.0..=8.0,
            state.separator_height,
            |x| format!("{:.1}px", x),
            Message::SeparatorHeight,
            None,
        ),
        labeled_slider(
            "Separator margin",
            0.0..=24.0,
            state.separator_margin_vertical,
            |x| format!("{:.0}px", x),
            Message::SeparatorMarginVertical,
            None,
        ),
    ]
    .spacing(8);

    let rows = column![
        text("Rows").size(14),
        labeled_slider(
            "Label size",
            8.0..=24.0,
            state.label_size,
            |x| format!("{:.1}px", x),
            Message::LabelSize,
            None,
        ),
        labeled_slider(
            "Row height",
            20.0..=50.0,
            state.row_height,
            |x| format!("{:.0}px", x),
            Message::RowHeight,
            None,
        ),
        labeled_slider(
            "Row spacing",
            0.0..=16.0,
            state.row_spacing,
            |x| format!("{:.0}px", x),
            Message::RowSpacing,
            None,
        ),
    ]
    .spacing(8);

    let icons = column![
        text("Icons (when enabled)").size(14),
        labeled_slider(
            "Icon slot width",
            8.0..=40.0,
            state.icon_slot_width,
            |x| format!("{:.0}px", x),
            Message::IconSlotWidth,
            None,
        ),
        labeled_slider(
            "Icon / label gap",
            0.0..=24.0,
            state.icon_label_gap,
            |x| format!("{:.0}px", x),
            Message::IconLabelGap,
            None,
        ),
        labeled_slider(
            "Glyph icon size",
            8.0..=28.0,
            state.icon_glyph_size,
            |x| format!("{:.0}px", x),
            Message::IconGlyphSize,
            None,
        ),
    ]
    .spacing(8);

    let hotkeys = column![
        text("Hotkey column").size(14),
        labeled_slider(
            "Hotkey label size",
            8.0..=20.0,
            state.hotkey_label_size,
            |x| format!("{:.1}px", x),
            Message::HotkeyLabelSize,
            None,
        ),
        labeled_slider(
            "Label / hotkey gap",
            0.0..=40.0,
            state.label_hotkey_gap,
            |x| format!("{:.0}px", x),
            Message::LabelHotkeyGap,
            None,
        ),
    ]
    .spacing(8);

    let submenus = column![
        text("Submenus").size(14),
        labeled_slider(
            "Flyout overlap",
            0.0..=100.0,
            state.submenu_flyout_overlap,
            |x| format!("{:.0}px", x),
            Message::SubmenuFlyoutOverlap,
            None,
        ),
        labeled_slider(
            "Chevron slot width",
            8.0..=40.0,
            state.submenu_chevron_slot_width,
            |x| format!("{:.0}px", x),
            Message::SubmenuChevronSlotWidth,
            None,
        ),
        text("Chevron icon").size(12),
        column![
            radio(
                "Arrow next (small)",
                SubmenuChevronIcon::ArrowNextSmall,
                Some(state.submenu_chevron_icon),
                Message::SubmenuChevronIcon,
            ),
            radio(
                "Arrow next",
                SubmenuChevronIcon::ArrowNext,
                Some(state.submenu_chevron_icon),
                Message::SubmenuChevronIcon,
            ),
            radio(
                "Arrow right (333)",
                SubmenuChevronIcon::ArrowRight333,
                Some(state.submenu_chevron_icon),
                Message::SubmenuChevronIcon,
            ),
            radio(
                "Arrow right (336)",
                SubmenuChevronIcon::ArrowRight336,
                Some(state.submenu_chevron_icon),
                Message::SubmenuChevronIcon,
            ),
        ]
        .spacing(4),
    ]
    .spacing(8);

    let appearance = column![
        theme,
        rule::horizontal(1),
        panel,
        rule::horizontal(1),
        rows,
        rule::horizontal(1),
        icons,
        rule::horizontal(1),
        hotkeys,
        rule::horizontal(1),
        submenus,
    ]
    .spacing(8);

    let controls = scrollable(
        column![behavior, rule::horizontal(1), appearance]
            .spacing(12)
            .padding(12),
    )
    .height(Length::Fill)
    .width(Length::Fill);

    let open_hint = match state.demo_open_mode {
        DemoOpenMode::RightClick => text("Right-click here").size(18),
        DemoOpenMode::Programmatic => text("Programmatic mode").size(18),
    };

    let maybe_open_btn = if state.demo_open_mode == DemoOpenMode::Programmatic {
        button("Open menu (API)").on_press(Message::RequestProgrammaticOpen)
    } else {
        button("Open menu (API)").on_press_maybe(None)
    };

    let target = container(
        column![
            open_hint,
            maybe_open_btn,
            text("Keyboard: arrows, Enter, Escape when open."),
            text(&state.status).size(14),
        ]
        .spacing(8),
    )
    .width(Length::FillPortion(1))
    .height(Length::Fill)
    .padding(24)
    .center_x(Length::Fill)
    .center_y(Length::Fill);

    let content = row![controls, rule::vertical(10), target].spacing(0);

    let open_mode = match state.demo_open_mode {
        DemoOpenMode::RightClick => ContextMenuOpen::RightClick,
        DemoOpenMode::Programmatic => ContextMenuOpen::Programmatic {
            open: state.programmatic_open_pulse,
            anchor: None,
        },
    };

    ContextMenu::new(content)
        .items(build_menu())
        .style(merged_style(state))
        .panel_padding(state.panel_padding)
        .min_width(state.min_width)
        .label_size(state.label_size)
        .row_height(state.row_height)
        .row_spacing(state.row_spacing)
        .border_radius(state.border_radius)
        .border_width(state.border_width)
        .submenu_flyout_overlap(state.submenu_flyout_overlap)
        .row_label_inset(state.row_label_inset)
        .submenu_chevron_icon(state.submenu_chevron_icon)
        .submenu_chevron_slot_width(state.submenu_chevron_slot_width)
        .icon_slot_width(state.icon_slot_width)
        .icon_label_gap(state.icon_label_gap)
        .icon_glyph_size(state.icon_glyph_size)
        .hotkey_label_size(state.hotkey_label_size)
        .label_hotkey_gap(state.label_hotkey_gap)
        .separator_height(state.separator_height)
        .separator_margin_vertical(state.separator_margin_vertical)
        .opens_with(open_mode)
        .on_open(Message::MenuOpened)
        .on_close(Message::MenuClosed)
        .on_select(Message::MenuSelected)
        .submenu_open_mode(state.submenu_mode)
        .show_item_icons(state.show_item_icons)
        .close_on_select(state.close_on_select)
        .into()
}
