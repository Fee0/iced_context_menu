//! Context menu widget and nested overlays.
//!
//! Submodules live in `src/context_menu/*.rs` (Rust `foo.rs` + `foo/` pattern).

mod menu;
mod style;
mod panel;
mod root_overlay;
mod state;
mod submenu_overlay;
mod widget;

pub use menu::{MenuItemId, MenuNode, MenuSpec};
pub use state::{ContextMenuState, SubmenuOpenMode};
pub use style::ContextMenuStyle;
pub use widget::ContextMenu;
