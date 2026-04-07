//! Configurable context menu overlay for [Iced](https://iced.rs): a [`Stack`](iced::widget::Stack) layer with a dismiss scrim and a positioned column of items.
//!
//! Use [`ContextMenuBuilder`] to define entries, [`context_menu_overlay`] with [`Stack::push_maybe`](iced::widget::Stack::push_maybe), and open the menu by storing [`ContextMenuOpen`] (for example on right-click).
//!
//! For [`MenuItem::Disabled`] rows, pass a dedicated `on_disabled_press` message (e.g. `Message::NoOp`) that your `update` ignores so clicks do not dismiss the menu.

mod context_menu;
mod style;

pub use context_menu::{
    context_menu_overlay, ContextMenuBuilder, ContextMenuOpen, MenuItem,
};
pub use style::ContextMenuStyle;
