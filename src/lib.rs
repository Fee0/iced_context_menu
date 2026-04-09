//! Customizable context menu for Iced.
//!
//! High-level API:
//! `ContextMenu::new(content).items(menu).on_open(...).on_close(...).on_select(...)`

mod context_menu;
mod menu;
mod style;

pub use context_menu::{ContextMenu, ContextMenuState, SubmenuOpenMode};
pub use menu::{MenuItemId, MenuNode, MenuSpec};
pub use style::ContextMenuStyle;
