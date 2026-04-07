use iced::widget::{column, container, text};
use iced::{Element, Length, Task};
use iced_context_menu::{ContextMenu, MenuItemId, MenuNode, MenuSpec, SubmenuOpenMode};

fn main() -> iced::Result {
    iced::application(|| State::default(), update, view).run()
}

#[derive(Debug, Clone)]
enum Message {
    MenuOpened,
    MenuClosed,
    MenuSelected(MenuItemId),
}

#[derive(Default)]
struct State {
    status: String,
}

fn build_menu() -> MenuSpec {
    MenuSpec::new()
        .action(1_u64, "Copy")
        .action(2_u64, "Paste")
        .separator()
        .disabled(3_u64, "Unavailable")
        .submenu(
            "More",
            vec![
                MenuNode::Action {
                    id: 4_u64.into(),
                    title: "Rename".into(),
                    enabled: true,
                },
                MenuNode::Submenu {
                    title: "Share".into(),
                    children: vec![
                        MenuNode::Action {
                            id: 5_u64.into(),
                            title: "Copy link".into(),
                            enabled: true,
                        },
                        MenuNode::Action {
                            id: 6_u64.into(),
                            title: "Open permissions".into(),
                            enabled: true,
                        },
                    ],
                },
            ],
        )
}

fn update(state: &mut State, message: Message) -> Task<Message> {
    match message {
        Message::MenuOpened => state.status = "Menu opened".to_string(),
        Message::MenuClosed => state.status = "Menu closed".to_string(),
        Message::MenuSelected(id) => {
            state.status = format!("Selected item {}", id);
        }
    }

    Task::none()
}

fn view(state: &State) -> Element<'_, Message> {
    let content = container(
        column![
            text("Right-click anywhere. Use arrows + Enter/Escape."),
            text(&state.status),
        ]
        .spacing(8),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .padding(24);

    ContextMenu::new(content)
        .items(build_menu())
        .on_open(Message::MenuOpened)
        .on_close(Message::MenuClosed)
        .on_select(Message::MenuSelected)
        .submenu_open_mode(SubmenuOpenMode::HoverAndClick)
        .submenu_hover_delay_ms(180)
        .close_on_select(true)
        .into()
}
