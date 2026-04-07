//! Stack-based overlay: dismiss scrim + positioned menu panels.

use crate::style::ContextMenuStyle;

use iced::alignment::{Horizontal, Vertical};
use iced::border::Radius;
use iced::widget::button::Status;
use iced::widget::{button, column, container, mouse_area, stack, text, Space};
use iced::{Background, Border, Color, Element, Length, Padding, Point, Rectangle, Theme};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ContextMenuOpen {
    pub at: Point,
}

#[derive(Clone, Debug)]
pub enum MenuItem<Message: Clone> {
    Action { label: String, message: Message },
    Separator,
    Disabled { label: String },
}

pub struct ContextMenuBuilder<Message: Clone> {
    items: Vec<MenuItem<Message>>,
}

impl<Message: Clone> ContextMenuBuilder<Message> {
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    pub fn push(mut self, title: impl Into<String>, message: Message) -> Self {
        self.items.push(MenuItem::Action {
            label: title.into(),
            message,
        });
        self
    }

    pub fn separator(mut self) -> Self {
        self.items.push(MenuItem::Separator);
        self
    }

    pub fn unavailable(mut self, title: impl Into<String>) -> Self {
        self.items.push(MenuItem::Disabled {
            label: title.into(),
        });
        self
    }

    pub fn build(self) -> Vec<MenuItem<Message>> {
        self.items
    }
}

impl<Message: Clone> Default for ContextMenuBuilder<Message> {
    fn default() -> Self {
        Self::new()
    }
}

pub fn estimate_panel_height<Message: Clone>(items: &[MenuItem<Message>], style: &ContextMenuStyle) -> f32 {
    let separators = items.iter().filter(|i| matches!(i, MenuItem::Separator)).count();
    let rows = items
        .iter()
        .filter(|i| matches!(i, MenuItem::Action { .. } | MenuItem::Disabled { .. }))
        .count();

    rows as f32 * style.row_height
        + separators as f32 * (style.separator_height + 2.0 * style.separator_margin_vertical)
        + style.panel_padding * 2.0
        + (rows.saturating_sub(1) + separators) as f32 * style.row_spacing
}

pub fn clamp_panel_anchor(
    mut at: Point,
    panel_width: f32,
    panel_height: f32,
    viewport: iced::Size,
) -> Point {
    let margin = 8.0;
    let max_x = (viewport.width - panel_width - margin).max(margin);
    let max_y = (viewport.height - panel_height - margin).max(margin);
    at.x = at.x.clamp(margin, max_x);
    at.y = at.y.clamp(margin, max_y);
    at
}

pub fn context_menu_overlay<'a, Message: Clone + 'a>(
    open: Option<ContextMenuOpen>,
    items: Vec<MenuItem<Message>>,
    on_dismiss: Message,
    on_inert_press: Message,
    viewport: iced::Size,
    style: ContextMenuStyle,
) -> Option<Element<'a, Message>> {
    let open = open?;
    let panel_height = estimate_panel_height(&items, &style);
    let anchor = clamp_panel_anchor(open.at, style.min_width.max(120.0), panel_height, viewport);

    Some(
        context_menu_overlay_panels(
            open,
            vec![(items, Rectangle::new(anchor, iced::Size::new(style.min_width, panel_height)))],
            on_dismiss,
            on_inert_press,
            style,
        )
        .into(),
    )
}

/// Full-window overlay: **scrim** (dismiss) + **N positioned panels** only—no extra filler layers.
/// Each panel is [`menu_panel`], which always yields a non-empty column (placeholder `Space` if `items` is empty).
pub fn context_menu_overlay_panels<'a, Message: Clone + 'a>(
    open: ContextMenuOpen,
    panels: Vec<(Vec<MenuItem<Message>>, Rectangle)>,
    on_dismiss: Message,
    on_inert_press: Message,
    style: ContextMenuStyle,
) -> Element<'a, Message> {
    let scrim_color = style.dismiss_scrim;
    let dismiss = mouse_area(
        container(Space::new().width(Length::Fill).height(Length::Fill)).style(
            move |_theme: &Theme| container::Style {
                background: Some(scrim_color.into()),
                ..Default::default()
            },
        ),
    )
    .on_press(on_dismiss);

    let mut layered = stack![dismiss].width(Length::Fill).height(Length::Fill);

    for (items, rect) in panels {
        let panel = menu_panel(items, style.clone(), on_inert_press.clone());
        let positioned = container(panel)
            .padding(Padding {
                top: rect.y,
                right: 0.0,
                bottom: 0.0,
                left: rect.x,
            })
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(Horizontal::Left)
            .align_y(Vertical::Top);
        layered = layered.push(positioned);
    }

    let _ = open;
    layered.into()
}

fn menu_panel<'a, Message: Clone + 'a>(
    items: Vec<MenuItem<Message>>,
    style: ContextMenuStyle,
    on_inert_press: Message,
) -> Element<'a, Message> {
    let mut col = column![].spacing(style.row_spacing);

    if items.is_empty() {
        col = col.push(
            Space::new()
                .width(Length::Fill)
                .height(Length::Fixed(1.0)),
        );
    }

    for item in items {
        match item {
            MenuItem::Action { label, message } => {
                col = col.push(action_row(label, message, style.clone()));
            }
            MenuItem::Separator => {
                let sep_color = style.separator_color;
                col = col.push(
                    mouse_area(
                        container(
                            Space::new()
                                .width(Length::Fill)
                                .height(Length::Fixed(style.separator_height)),
                        )
                        .padding(Padding {
                            top: style.separator_margin_vertical,
                            right: 0.0,
                            bottom: style.separator_margin_vertical,
                            left: 0.0,
                        })
                        .style(move |_theme: &Theme| container::Style {
                            background: Some(sep_color.into()),
                            ..Default::default()
                        }),
                    )
                    .on_press(on_inert_press.clone()),
                );
            }
            MenuItem::Disabled { label } => {
                let disabled_color = style.disabled_color;
                col = col.push(
                    mouse_area(
                        container(
                            text(label)
                                .size(style.label_size)
                                .style(move |_theme: &Theme| text::Style {
                                    color: Some(disabled_color),
                                }),
                        )
                        .height(Length::Fixed(style.row_height))
                        .padding([0.0, 8.0])
                        .width(Length::Fill),
                    )
                    .on_press(on_inert_press.clone()),
                );
            }
        }
    }

    let panel_bg = style.panel_background;
    let border = style.panel_border();
    let shadow = style.panel_shadow();

    let _ = on_inert_press;
    container(col)
        .padding(style.panel_padding)
        .width(Length::Fixed(style.min_width))
        .style(move |_theme: &Theme| container::Style {
            background: Some(panel_bg.into()),
            border,
            shadow,
            ..Default::default()
        })
        .into()
}

fn action_row<'a, Message: Clone + 'a>(
    label: String,
    message: Message,
    style: ContextMenuStyle,
) -> Element<'a, Message> {
    let label_color = style.label_color;
    button(
        text(label)
            .size(style.label_size)
            .style(move |_theme: &Theme| text::Style {
                color: Some(label_color),
            }),
    )
    .height(Length::Fixed(style.row_height))
    .width(Length::Fill)
    .padding([0.0, 8.0])
    .style(move |_theme: &Theme, status| {
        let base = iced::widget::button::Style {
            background: Some(Background::Color(Color::TRANSPARENT)),
            text_color: label_color,
            border: Border {
                radius: Radius::from(4.0),
                ..Default::default()
            },
            ..Default::default()
        };
        match status {
            Status::Hovered => iced::widget::button::Style {
                background: Some(Background::Color(Color::from_rgb(0.22, 0.24, 0.28))),
                ..base
            },
            Status::Pressed => iced::widget::button::Style {
                background: Some(Background::Color(Color::from_rgb(0.18, 0.2, 0.24))),
                ..base
            },
            _ => base,
        }
    })
    .on_press(message)
    .into()
}
