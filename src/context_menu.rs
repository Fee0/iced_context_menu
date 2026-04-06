//! Stack-based overlay: dismiss scrim + positioned column of items.

use crate::style::ContextMenuStyle;

use iced::alignment::{Horizontal, Vertical};
use iced::border::Radius;
use iced::widget::{button, column, container, mouse_area, stack, text, Space};
use iced::widget::button::Status;
use iced::{Background, Border, Color, Element, Length, Padding, Point, Theme};

/// When set, a context menu is shown with its top-left near this anchor (logical pixels).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ContextMenuOpen {
    pub at: Point,
}

/// One row in a context menu: action, separator, or disabled label.
#[derive(Clone, Debug)]
pub enum MenuItem<Message: Clone> {
    Action {
        label: &'static str,
        message: Message,
    },
    Separator,
    Disabled {
        label: &'static str,
    },
}

/// Full-screen overlay: dimmed dismiss layer and a positioned menu column.
/// Returns [`None`] when `open` is [`None`] so callers can use [`Stack::push_maybe`](iced::widget::Stack::push_maybe).
pub fn context_menu_overlay<'a, Message: Clone + 'a>(
    open: Option<ContextMenuOpen>,
    items: &'a [MenuItem<Message>],
    on_dismiss: Message,
    viewport: iced::Size,
    style: &'a ContextMenuStyle,
) -> Option<Element<'a, Message>> {
    let open = open?;
    let anchor = clamp_anchor(open.at, items, viewport, style);

    let dismiss = mouse_area(
        container(Space::new(Length::Fill, Length::Fill)).style(
            move |_theme: &Theme| container::Style {
                background: Some(style.dismiss_scrim.into()),
                ..Default::default()
            },
        ),
    )
    .on_press(on_dismiss.clone());

    let panel = menu_panel(items, style);

    let positioned = container(panel)
        .padding(Padding {
            top: anchor.y,
            right: 0.0,
            bottom: 0.0,
            left: anchor.x,
        })
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(Horizontal::Left)
        .align_y(Vertical::Top);

    let layers = stack![dismiss, positioned]
        .width(Length::Fill)
        .height(Length::Fill);

    Some(layers.into())
}

fn clamp_anchor<Message: Clone>(
    mut at: Point,
    items: &[MenuItem<Message>],
    viewport: iced::Size,
    style: &ContextMenuStyle,
) -> Point {
    let margin = 8.0;
    let w = style.min_width.max(120.0);
    let separators = items.iter().filter(|i| matches!(i, MenuItem::Separator)).count();
    let rows = items
        .iter()
        .filter(|i| matches!(i, MenuItem::Action { .. } | MenuItem::Disabled { .. }))
        .count();
    let h = rows as f32 * style.row_height
        + separators as f32 * (style.separator_height + 2.0 * style.separator_margin_vertical)
        + style.panel_padding * 2.0
        + (rows.saturating_sub(1) + separators) as f32 * style.row_spacing;

    let max_x = (viewport.width - w - margin).max(margin);
    let max_y = (viewport.height - h - margin).max(margin);
    at.x = at.x.clamp(margin, max_x);
    at.y = at.y.clamp(margin, max_y);
    at
}

fn menu_panel<'a, Message: Clone + 'a>(
    items: &'a [MenuItem<Message>],
    style: &'a ContextMenuStyle,
) -> Element<'a, Message> {
    let mut col = column![].spacing(style.row_spacing);

    for item in items {
        match item {
            MenuItem::Action { label, message } => {
                col = col.push(action_row(label, message.clone(), style));
            }
            MenuItem::Separator => {
                col = col.push(
                    container(Space::new(Length::Fill, Length::Fixed(style.separator_height)))
                        .padding(Padding {
                            top: style.separator_margin_vertical,
                            right: 0.0,
                            bottom: style.separator_margin_vertical,
                            left: 0.0,
                        })
                        .style(move |_theme: &Theme| container::Style {
                            background: Some(style.separator_color.into()),
                            ..Default::default()
                        }),
                );
            }
            MenuItem::Disabled { label } => {
                col = col.push(
                    container(
                        text(*label)
                            .size(style.label_size)
                            .style(move |_theme: &Theme| text::Style {
                                color: Some(style.disabled_color),
                            }),
                    )
                    .padding([4.0, 8.0])
                    .width(Length::Fill),
                );
            }
        }
    }

    container(col)
        .padding(style.panel_padding)
        .width(Length::Fixed(style.min_width))
        .style(move |_theme: &Theme| container::Style {
            background: Some(style.panel_background.into()),
            border: style.panel_border(),
            shadow: style.panel_shadow(),
            ..Default::default()
        })
        .into()
}

fn action_row<'a, Message: Clone + 'a>(
    label: &'static str,
    message: Message,
    style: &'a ContextMenuStyle,
) -> Element<'a, Message> {
    button(
        text(label)
            .size(style.label_size)
            .style(move |_theme: &Theme| text::Style {
                color: Some(style.label_color),
            }),
    )
    .width(Length::Fill)
    .padding([4.0, 8.0])
    .style(move |_theme: &Theme, status| {
        let base = iced::widget::button::Style {
            background: Some(Background::Color(Color::TRANSPARENT)),
            text_color: style.label_color,
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
