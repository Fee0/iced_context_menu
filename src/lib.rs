//! Customizable context menu for Iced.
//!
//! High-level API:
//! `ContextMenu::new(content).items(menu).on_open(...).on_close(...).on_select(...)`

mod context_menu;
mod menu;
mod style;
mod widget;

pub use menu::{MenuItemId, MenuNode, MenuSpec};
pub use style::ContextMenuStyle;
pub use widget::{ContextMenu, SubmenuOpenMode};

pub mod advanced {
    //! Unstable low-level APIs. Prefer using `ContextMenu`.
    pub use crate::context_menu::{
        ContextMenuBuilder, ContextMenuOpen, MenuItem, context_menu_overlay,
    };
}
