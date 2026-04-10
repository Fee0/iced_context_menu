//! Context menu widget and nested overlays.
//!
//! Submodules live in `src/context_menu/*.rs` (Rust `foo.rs` + `foo/` pattern).

mod menu;
mod menu_overlay;
mod panel;
mod state;
mod style;
pub mod submenu_chevron;
mod widget;

pub use menu::{MenuIcon, MenuItemId, MenuNode, MenuSpec};
pub use state::{ContextMenuState, SubmenuOpenMode};
pub use style::ContextMenuStyle;
pub use widget::ContextMenu;
