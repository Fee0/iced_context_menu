//! Configurable context menu overlay for [Iced](https://iced.rs): a [`Stack`](iced::widget::Stack) layer with a dismiss scrim and a positioned column of items.
//!
//! Use [`ContextMenuBuilder`] to define entries, [`context_menu_overlay`] with [`Stack::push_maybe`](iced::widget::Stack::push_maybe), and open the menu by storing [`ContextMenuOpen`] (for example on right-click).
//!
//! Pass `on_inert_press` (e.g. `Message::NoOp`) for presses on non-action areas (separator, disabled row, panel chrome); your `update` should ignore it so the menu stays open.

mod context_menu;
mod style;

pub use context_menu::{ContextMenuBuilder, ContextMenuOpen, MenuItem, context_menu_overlay};
pub use style::ContextMenuStyle;
