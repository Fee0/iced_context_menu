//! Custom widget with coordinate-only regions: one `Element` draws a split panel and hit-tests
//! right-clicks. [`ContextMenuOpen::Programmatic`](iced_context_menu::ContextMenuOpen::Programmatic)
//! opens a different [`MenuSpec`](iced_context_menu::MenuSpec) per half (no per-cell `Element`s).

use iced::advanced::layout;
use iced::advanced::renderer;
use iced::advanced::widget::Widget;
use iced::advanced::{Clipboard, Renderer, Shell};
use iced::mouse;
use iced::widget::{column, container, text};
use iced::window::Settings;
use iced::{
    Border, Color, Element, Event, Length, Point, Rectangle, Size, Task, Theme,
};
use iced_context_menu::{
    ContextMenu, ContextMenuOpen, ContextMenuStyle, MenuItemId, MenuSpec,
};

fn main() -> iced::Result {
    iced::application(|| State::default(), update, view)
        .window(Settings {
            size: Size::new(720.0, 480.0),
            ..Settings::default()
        })
        .run()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Half {
    Left,
    Right,
}

#[derive(Debug, Clone)]
enum Message {
    OpenContextMenu { half: Half, at: Point },
    MenuOpened,
    MenuClosed,
    MenuSelected(MenuItemId),
}

#[derive(Debug, Clone)]
struct State {
    status: String,
    /// Which half the next open menu is for (`view` builds `MenuSpec` from this).
    menu_half: Half,
    /// One-shot open request for [`ContextMenuOpen::Programmatic`].
    open_pulse: bool,
    menu_anchor: Point,
}

impl Default for State {
    fn default() -> Self {
        Self {
            status: "Right-click the left or right side of the panel.".to_string(),
            menu_half: Half::Left,
            open_pulse: false,
            menu_anchor: Point::ORIGIN,
        }
    }
}

fn menu_for_half(half: Half) -> MenuSpec<'static> {
    match half {
        Half::Left => MenuSpec::new()
            .action(10_u64, "Left: Alpha", None, None)
            .action(11_u64, "Left: Beta", None, None),
        Half::Right => MenuSpec::new()
            .action(20_u64, "Right: One", None, None)
            .action(21_u64, "Right: Two", None, None)
            .separator()
            .action(22_u64, "Right: Three", None, None),
    }
}

fn update(state: &mut State, message: Message) -> Task<Message> {
    match message {
        Message::OpenContextMenu { half, at } => {
            state.menu_half = half;
            state.menu_anchor = at;
            state.open_pulse = true;
        }
        Message::MenuOpened => {
            state.open_pulse = false;
            state.status = format!("Menu opened ({half:?} region).", half = state.menu_half);
        }
        Message::MenuClosed => {
            state.status = "Menu closed.".to_string();
        }
        Message::MenuSelected(id) => {
            state.status = format!(
                "Selected item {} (last menu was {:?}).",
                id, state.menu_half
            );
        }
    }
    Task::none()
}

fn view(state: &State) -> Element<'_, Message> {
    let content = column![
        text("Two-region hit test").size(20),
        text(&state.status).size(14),
        ContextMenu::new(Element::new(SplitHitPanel))
            .items(menu_for_half(state.menu_half))
            .style(ContextMenuStyle::light())
            .opens_with(ContextMenuOpen::Programmatic {
                open: state.open_pulse,
                anchor: Some(state.menu_anchor),
            })
            .on_open(Message::MenuOpened)
            .on_close(Message::MenuClosed)
            .on_select(Message::MenuSelected),
    ]
    .spacing(12)
    .padding(24);

    container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .into()
}

/// Stateless panel: paints a vertical divider and tints each half; hit-tests in [`Widget::update`].
struct SplitHitPanel;

impl Widget<Message, Theme, iced::Renderer> for SplitHitPanel {
    fn size(&self) -> Size<Length> {
        Size {
            width: Length::Fill,
            height: Length::Fixed(220.0),
        }
    }

    fn layout(
        &mut self,
        _tree: &mut iced::advanced::widget::Tree,
        _renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let size = limits
            .width(Length::Fill)
            .height(Length::Fixed(220.0))
            .resolve(Length::Fill, Length::Fixed(220.0), Size::ZERO);
        layout::Node::new(size)
    }

    fn draw(
        &self,
        _tree: &iced::advanced::widget::Tree,
        renderer: &mut iced::Renderer,
        _theme: &Theme,
        _style: &renderer::Style,
        layout: iced::advanced::Layout<'_>,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) {
        let b = layout.bounds();
        let mid = b.x + b.width * 0.5;

        let left_bg = Color::from_rgba(0.2, 0.45, 0.85, 0.12);
        let right_bg = Color::from_rgba(0.2, 0.75, 0.35, 0.12);
        let line = Color::from_rgb(0.35, 0.35, 0.4);
        let border_c = Color::from_rgb(0.5, 0.5, 0.55);

        renderer.fill_quad(
            renderer::Quad {
                bounds: Rectangle {
                    x: b.x,
                    y: b.y,
                    width: b.width * 0.5,
                    height: b.height,
                },
                ..renderer::Quad::default()
            },
            left_bg,
        );
        renderer.fill_quad(
            renderer::Quad {
                bounds: Rectangle {
                    x: mid,
                    y: b.y,
                    width: b.width * 0.5,
                    height: b.height,
                },
                ..renderer::Quad::default()
            },
            right_bg,
        );

        let line_w = 2.0;
        renderer.fill_quad(
            renderer::Quad {
                bounds: Rectangle {
                    x: mid - line_w * 0.5,
                    y: b.y,
                    width: line_w,
                    height: b.height,
                },
                ..renderer::Quad::default()
            },
            line,
        );

        renderer.fill_quad(
            renderer::Quad {
                bounds: b,
                border: Border {
                    width: 1.0,
                    color: border_c,
                    radius: 6.0.into(),
                },
                ..renderer::Quad::default()
            },
            Color::TRANSPARENT,
        );
    }

    fn update(
        &mut self,
        _tree: &mut iced::advanced::widget::Tree,
        event: &Event,
        layout: iced::advanced::Layout<'_>,
        cursor: mouse::Cursor,
        _renderer: &iced::Renderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) {
        let Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right)) = event else {
            return;
        };
        let Some(pos) = cursor.position() else {
            return;
        };
        let b = layout.bounds();
        if !b.contains(pos) {
            return;
        }

        let half = if pos.x < b.x + b.width * 0.5 {
            Half::Left
        } else {
            Half::Right
        };

        shell.publish(Message::OpenContextMenu { half, at: pos });
        shell.capture_event();
    }

    fn mouse_interaction(
        &self,
        _tree: &iced::advanced::widget::Tree,
        layout: iced::advanced::Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        if cursor.is_over(layout.bounds()) {
            mouse::Interaction::Pointer
        } else {
            mouse::Interaction::default()
        }
    }
}
