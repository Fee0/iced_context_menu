//! Opens a context menu on right-click using [`mouse_area`](iced::widget::MouseArea).
//! Cursor position is tracked with [`on_move`](iced::widget::MouseArea::on_move) because
//! [`on_right_press`](iced::widget::MouseArea::on_right_press) does not carry a point in Iced 0.14.
//! Press Escape or click the dimmed area to dismiss.

use iced::keyboard;
use iced::widget::{Stack, center, column, container, mouse_area, text};
use iced::{Element, Length, Point, Size, Task};
use iced_context_menu::{
    ContextMenuBuilder, ContextMenuOpen, ContextMenuStyle, MenuItem, context_menu_overlay,
};

fn main() -> iced::Result {
    iced::application(|| State::default(), update, view)
        .subscription(|_| {
            iced::Subscription::batch([
                keyboard::listen().filter_map(|event| match event {
                    keyboard::Event::KeyPressed { key, .. } => match key {
                        keyboard::Key::Named(keyboard::key::Named::Escape) => {
                            Some(Message::CloseMenu)
                        }
                        _ => None,
                    },
                    _ => None,
                }),
                iced::window::resize_events().map(|(_id, size)| Message::WindowResized(size)),
            ])
        })
        .run()
}

#[derive(Debug, Clone)]
enum Message {
    CursorMoved(Point),
    OpenMenu,
    CloseMenu,
    /// Ignored: used as `on_inert_press` for non-action parts of the menu panel.
    NoOp,
    Copy,
    Paste,
    WindowResized(Size),
}

struct State {
    open: Option<ContextMenuOpen>,
    /// Last pointer position inside the [`mouse_area`] (logical pixels).
    cursor: Point,
    viewport: Size,
    status: String,
    menu_items: Vec<MenuItem<Message>>,
    menu_style: ContextMenuStyle,
}

fn build_context_menu_state() -> Vec<MenuItem<Message>> {
    ContextMenuBuilder::new()
        .push("Copy", Message::Copy)
        .push("Paste", Message::Paste)
        .separator()
        .unavailable("Unavailable")
        .build()
}

impl Default for State {
    fn default() -> Self {
        Self {
            open: None,
            cursor: Point::ORIGIN,
            viewport: Size::new(800.0, 600.0),
            status: String::from("Right-click the area."),
            menu_items: build_context_menu_state(),
            menu_style: ContextMenuStyle::default(),
        }
    }
}

fn update(state: &mut State, message: Message) -> Task<Message> {
    match message {
        Message::CursorMoved(p) => state.cursor = p,
        Message::OpenMenu => {
            state.open = Some(ContextMenuOpen { at: state.cursor });
        }
        Message::CloseMenu => state.open = None,
        Message::NoOp => {}
        Message::Copy => {
            state.open = None;
            state.status = "Copy (demo).".to_string();
        }
        Message::Paste => {
            state.open = None;
            state.status = "Paste (demo).".to_string();
        }
        Message::WindowResized(s) => state.viewport = s,
    }
    Task::none()
}

fn view(state: &State) -> Element<Message> {
    let body = center(
        container(
            column![
                text(&state.status).size(16),
                text("Right-click here.").size(14),
            ]
            .spacing(8),
        )
        .padding(24),
    );

    let content = mouse_area(container(body).width(Length::Fill).height(Length::Fill))
        .on_move(Message::CursorMoved)
        .on_right_press(Message::OpenMenu);

    let base = container(content).width(Length::Fill).height(Length::Fill);
    let overlay = context_menu_overlay(
        state.open,
        &state.menu_items,
        Message::CloseMenu,
        Message::NoOp,
        state.viewport,
        &state.menu_style,
    );

    match overlay {
        Some(layer) => Stack::new().push(base).push(layer).into(),
        None => Stack::new().push(base).into(),
    }
}
