//! Customizable context menu for Iced.
//!
//! # Quick start
//!
//! ```ignore
//! ContextMenu::new(content)
//!     .items(menu)
//!     .on_open(...)
//!     .on_close(...)
//!     .on_select(...)
//! ```
//!
//! # Theming
//!
//! Use [`ContextMenuStyle`] for full control (colors, typography, separators, submenu chevron,
//! flyout overlap, panel shadow, scrim). Presets [`ContextMenuStyle::example_dark`],
//! [`ContextMenuStyle::example_light`], and [`ContextMenuStyle::example_warm`] are starting points.
//!
//! [`ContextMenu`] also exposes builder shortcuts for common layout fields (padding, row size,
//! borders, hotkey column, icon column, shadow, etc.). Anything without a dedicated method can be
//! set on a [`ContextMenuStyle`] value before calling [`.style(...)`](ContextMenu::style).
//!
//! # Menu data
//!
//! [`MenuSpec`] and [`MenuNode`] use a lifetime and [`std::borrow::Cow`] for row titles and hotkey
//! text so you can borrow `&str` from application state instead of allocating every frame. Build
//! or replace the spec when the underlying data changes (typical Iced pattern). String literals and
//! [`String`] still work via `.into()`.
//!
//! # State
//!
//! [`ContextMenuState`] is stored in the widget tree. Its fields are useful for **observing** whether
//! the menu is open and where it is anchored; changing them from outside the widget is not supported.

mod context_menu;

pub use context_menu::submenu_chevron::SubmenuChevronIcon;
pub use context_menu::{
    ContextMenu, ContextMenuState, ContextMenuStyle, MenuIcon, MenuItemId, MenuNode, MenuSpec,
    SubmenuOpenMode,
};
