use iced::widget::{
    button, checkbox, column, container, radio, row, rule, scrollable, slider, text,
};
use iced::{Color, Element, Length, Task};
use iced_context_menu::{
    ContextMenu, ContextMenuOpen, ContextMenuStyle, MenuIcon, MenuItemId, MenuSpec, SubmenuOpenMode,
};

fn main() -> iced::Result {
    iced::application(|| State::default(), update, view).run()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StylePreset {
    Dark,
    Light,
    Warm,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum DemoOpenMode {
    #[default]
    RightClick,
    Programmatic,
}

fn merged_style(state: &State) -> ContextMenuStyle {
    let mut s = match state.style_preset {
        StylePreset::Dark => ContextMenuStyle::example_dark(),
        StylePreset::Light => ContextMenuStyle::example_light(),
        StylePreset::Warm => ContextMenuStyle::example_warm(),
    };
    s.panel_padding = state.panel_padding;
    s.min_width = state.min_width;
    s.label_size = state.label_size;
    s.row_height = state.row_height;
    s.row_spacing = state.row_spacing;
    s.border_radius = state.border_radius;
    s.border_width = state.border_width;
    s.submenu_flyout_overlap = state.submenu_flyout_overlap;
    s.panel_shadow.blur_radius = state.panel_shadow_blur;
    s.dismiss_scrim = Color::from_rgba(0.0, 0.0, 0.0, state.scrim_alpha);
    s
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
    StylePreset(StylePreset),
    LongLabel(bool),
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
    style_preset: StylePreset,
    long_label: bool,
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
            style_preset: StylePreset::Dark,
            long_label: false,
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

fn build_menu(long_label: bool) -> MenuSpec<'static> {
    let copy_title: String = if long_label {
        "Copy (long label to exercise min width)".into()
    } else {
        "Copy".into()
    };

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
            copy_title,
            Some(demo_row_icon()),
            Some("Ctrl+C".into()),
        )
        .action(2_u64, "Paste", None, Some("Ctrl+V".into()))
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
        Message::StylePreset(p) => state.style_preset = p,
        Message::LongLabel(v) => state.long_label = v,
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
        text("Behavior").size(16),
        text("How nested submenus open:"),
        column![
            radio(
                "Hover — open as soon as pointer enters row",
                SubmenuOpenMode::Hover,
                Some(state.submenu_mode),
                Message::SubmenuMode,
            ),
            radio(
                "Click — open submenu on click",
                SubmenuOpenMode::Click,
                Some(state.submenu_mode),
                Message::SubmenuMode,
            ),
        ]
        .spacing(4),
        checkbox(state.show_item_icons)
            .label("Show icons")
            .on_toggle(Message::ShowItemIcons),
        checkbox(state.close_on_select)
            .label("Close menu after selecting an action")
            .on_toggle(Message::CloseOnSelect),
        checkbox(state.long_label)
            .label("Long first row label")
            .on_toggle(Message::LongLabel),
        text("How the root menu opens:"),
        column![
            radio(
                "Right-click on target (default)",
                DemoOpenMode::RightClick,
                Some(state.demo_open_mode),
                Message::DemoOpenMode,
            ),
            radio(
                "Programmatic only — use button on target",
                DemoOpenMode::Programmatic,
                Some(state.demo_open_mode),
                Message::DemoOpenMode,
            ),
        ]
        .spacing(4),
    ]
    .spacing(8);

    let appearance = column![
        text("Appearance").size(16),
        text("Example style preset"),
        column![
            radio(
                "Dark (default)",
                StylePreset::Dark,
                Some(state.style_preset),
                Message::StylePreset,
            ),
            radio(
                "Light panel",
                StylePreset::Light,
                Some(state.style_preset),
                Message::StylePreset,
            ),
            radio(
                "Warm",
                StylePreset::Warm,
                Some(state.style_preset),
                Message::StylePreset,
            ),
        ]
        .spacing(4),
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
            "Label size",
            0.0..=50.0,
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
            0.0..=30.0,
            state.row_spacing,
            |x| format!("{:.0}px", x),
            Message::RowSpacing,
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
            "Submenu flyout overlap",
            0.0..=100.0,
            state.submenu_flyout_overlap,
            |x| format!("{:.0}px", x),
            Message::SubmenuFlyoutOverlap,
            None,
        ),
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
    ]
    .spacing(8);

    let controls = scrollable(
        column![
            text("Context menu settings").size(20),
            text("Adjust values, then use the target area (right-click or programmatic open)."),
            rule::horizontal(10),
            behavior,
            rule::horizontal(10),
            appearance,
        ]
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
        .items(build_menu(state.long_label))
        .style(merged_style(state))
        .opens_with(open_mode)
        .on_open(Message::MenuOpened)
        .on_close(Message::MenuClosed)
        .on_select(Message::MenuSelected)
        .submenu_open_mode(state.submenu_mode)
        .show_item_icons(state.show_item_icons)
        .close_on_select(state.close_on_select)
        .into()
}
