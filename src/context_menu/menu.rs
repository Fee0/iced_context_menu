//! Hierarchical menu description for [`crate::ContextMenu`].
//!
//! # Menu data and lifetimes
//!
//! [`MenuSpec`] and [`MenuNode`] carry a lifetime `'a` on text fields stored as [`Cow`]: you can use
//! `Cow::Borrowed` for `&str` slices from app state (no per-frame allocation), or `Cow::Owned` /
//! `.into()` from [`String`] or string literals. Prefer **building or updating** a [`MenuSpec`] when
//! the underlying data changes, not necessarily on every `view()` tick.
//!
//! Row virtualization is not provided; very large menus pay full layout cost for all rows.

use std::borrow::Cow;
use std::fmt;

use iced::advanced::svg;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MenuItemId(pub u64);

impl fmt::Display for MenuItemId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// SVG icon shown to the left of a row label when [`crate::ContextMenu::show_item_icons`] is true.
#[derive(Debug, Clone)]
pub struct MenuIcon(svg::Handle);

impl MenuIcon {
    /// Build from raw SVG bytes; use `include_bytes!` at the call site for embedded assets.
    pub fn from_svg_bytes(bytes: impl Into<Cow<'static, [u8]>>) -> Self {
        Self(svg::Handle::from_memory(bytes.into()))
    }

    pub(crate) fn handle(&self) -> svg::Handle {
        self.0.clone()
    }
}

#[derive(Debug, Clone)]
pub enum MenuNode<'a> {
    Action {
        id: MenuItemId,
        title: Cow<'a, str>,
        enabled: bool,
        icon: Option<MenuIcon>,
        /// Display-only shortcut hint (e.g. `"Ctrl+S"`). Shown right-aligned when set.
        hotkey: Option<Cow<'a, str>>,
    },
    Separator,
    Submenu {
        title: Cow<'a, str>,
        children: Vec<MenuNode<'a>>,
        icon: Option<MenuIcon>,
    },
}

#[derive(Debug, Clone)]
pub struct MenuSpec<'a> {
    nodes: Vec<MenuNode<'a>>,
}

impl<'a> Default for MenuSpec<'a> {
    fn default() -> Self {
        Self { nodes: Vec::new() }
    }
}

impl<'a> MenuSpec<'a> {
    pub fn new() -> Self {
        Self { nodes: Vec::new() }
    }

    pub fn action(
        mut self,
        id: impl Into<MenuItemId>,
        title: impl Into<Cow<'a, str>>,
        icon: Option<MenuIcon>,
        hotkey: Option<Cow<'a, str>>,
    ) -> Self {
        self.nodes.push(MenuNode::Action {
            id: id.into(),
            title: title.into(),
            enabled: true,
            icon,
            hotkey,
        });
        self
    }

    pub fn disabled(
        mut self,
        id: impl Into<MenuItemId>,
        title: impl Into<Cow<'a, str>>,
        icon: Option<MenuIcon>,
        hotkey: Option<Cow<'a, str>>,
    ) -> Self {
        self.nodes.push(MenuNode::Action {
            id: id.into(),
            title: title.into(),
            enabled: false,
            icon,
            hotkey,
        });
        self
    }

    pub fn separator(mut self) -> Self {
        self.nodes.push(MenuNode::Separator);
        self
    }

    pub fn submenu(
        mut self,
        title: impl Into<Cow<'a, str>>,
        children: impl Into<Vec<MenuNode<'a>>>,
        icon: Option<MenuIcon>,
    ) -> Self {
        self.nodes.push(MenuNode::Submenu {
            title: title.into(),
            children: children.into(),
            icon,
        });
        self
    }

    pub fn nodes(&self) -> &[MenuNode<'a>] {
        &self.nodes
    }
}

impl From<u64> for MenuItemId {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

impl<'a> From<Vec<MenuNode<'a>>> for MenuSpec<'a> {
    fn from(nodes: Vec<MenuNode<'a>>) -> Self {
        Self { nodes }
    }
}
