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
//! # Opening the menu
//!
//! Default behavior is right-click on the widget. For a parent-driven open, call
//! [`.opens_with`](ContextMenu::opens_with) with [`ContextMenuOpen::Programmatic`] each view:
//! set `open: true` for one update (then clear it once the menu has opened—for example when
//! handling the message from [`.on_open`](ContextMenu::on_open))—so the request is a one-shot pulse,
//! like other Iced UI flags.
//!
//! # Theming and layout
//!
//! Use [`ContextMenuStyle`] for colors, shadow, and dismiss scrim. For palette-aligned colors,
//! use [`ContextMenuStyle::from_theme`], [`ContextMenuStyle::dark`], or [`ContextMenuStyle::light`].
//!
//! Spacing, sizing, typography, borders, icon columns, submenu chevron, and flyout overlap are
//! configured on [`ContextMenu`] via builder methods such as [`.panel_padding`](ContextMenu::panel_padding)
//! and [`.row_height`](ContextMenu::row_height). [`.style(...)`](ContextMenu::style) replaces the style
//! only; layout fields on the widget are unchanged.
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
    ContextMenu, ContextMenuOpen, ContextMenuState, ContextMenuStyle, MenuIcon, MenuItemId,
    MenuNode, MenuSpec, SubmenuOpenMode,
};
pub use iced::advanced::text::Shaping;
