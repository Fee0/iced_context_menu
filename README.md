# iced_context_menu

Customizable context menus for [Iced](https://github.com/iced-rs/iced) **0.14**: right-click or programmatic open,
nested submenus, optional SVG row icons, hotkey hints, and theme-aware styling.

![Nested context menus with icons, hotkeys, disabled row, and light theme](assets/screenshot.png)

## Usage

Add to `Cargo.toml`:

```toml
iced_context_menu = { git = "https://github.com/Fee0/iced_context_menu.git" }
```

Wrap any widget and supply a `MenuSpec`. Build with `.action`, `.disabled`, `.separator`, `.submenu`.

```rust
use iced::widget::text;
use iced_context_menu::{ContextMenu, ContextMenuStyle, MenuSpec};

fn view() -> iced::Element<'_, Message> {
    ContextMenu::new(text("Right-click me"))
        .items(
            MenuSpec::new()
                .action(1_u64, "Copy", None, Some("Ctrl+C".into()))
                .separator()
                .disabled(2_u64, "Unavailable", None, None),
        )
        .style(ContextMenuStyle::light())
        .on_select(|id| Message::MenuItem(id))
        .into()
}
```

Open behavior defaults to **right-click** on the wrapped region. Programmatic open also supported.

## Examples

Configure and test your context menu:

```bash
cargo run --example right_click
```
