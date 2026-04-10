//! Customizable context menu for Iced.
//!
//! High-level API:
//! `ContextMenu::new(content).items(menu).on_open(...).on_close(...).on_select(...)`

mod context_menu;

pub use context_menu::submenu_chevron::SubmenuChevronIcon;
pub use context_menu::{
    ContextMenu, ContextMenuState, ContextMenuStyle, MenuItemId, MenuNode, MenuSpec,
    SubmenuOpenMode,
};
