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
pub enum MenuNode {
    Action {
        id: MenuItemId,
        title: String,
        enabled: bool,
        icon: Option<MenuIcon>,
    },
    Separator,
    Submenu {
        title: String,
        children: Vec<MenuNode>,
        icon: Option<MenuIcon>,
    },
}

#[derive(Debug, Clone, Default)]
pub struct MenuSpec {
    nodes: Vec<MenuNode>,
}

impl MenuSpec {
    pub fn new() -> Self {
        Self { nodes: Vec::new() }
    }

    pub fn action(mut self, id: impl Into<MenuItemId>, title: impl Into<String>) -> Self {
        self.nodes.push(MenuNode::Action {
            id: id.into(),
            title: title.into(),
            enabled: true,
            icon: None,
        });
        self
    }

    pub fn action_with_icon(
        mut self,
        id: impl Into<MenuItemId>,
        title: impl Into<String>,
        icon: MenuIcon,
    ) -> Self {
        self.nodes.push(MenuNode::Action {
            id: id.into(),
            title: title.into(),
            enabled: true,
            icon: Some(icon),
        });
        self
    }

    pub fn disabled(mut self, id: impl Into<MenuItemId>, title: impl Into<String>) -> Self {
        self.nodes.push(MenuNode::Action {
            id: id.into(),
            title: title.into(),
            enabled: false,
            icon: None,
        });
        self
    }

    pub fn disabled_with_icon(
        mut self,
        id: impl Into<MenuItemId>,
        title: impl Into<String>,
        icon: MenuIcon,
    ) -> Self {
        self.nodes.push(MenuNode::Action {
            id: id.into(),
            title: title.into(),
            enabled: false,
            icon: Some(icon),
        });
        self
    }

    pub fn separator(mut self) -> Self {
        self.nodes.push(MenuNode::Separator);
        self
    }

    pub fn submenu(
        mut self,
        title: impl Into<String>,
        children: impl Into<Vec<MenuNode>>,
    ) -> Self {
        self.nodes.push(MenuNode::Submenu {
            title: title.into(),
            children: children.into(),
            icon: None,
        });
        self
    }

    pub fn submenu_with_icon(
        mut self,
        title: impl Into<String>,
        icon: MenuIcon,
        children: impl Into<Vec<MenuNode>>,
    ) -> Self {
        self.nodes.push(MenuNode::Submenu {
            title: title.into(),
            children: children.into(),
            icon: Some(icon),
        });
        self
    }

    pub fn nodes(&self) -> &[MenuNode] {
        &self.nodes
    }
}

impl From<u64> for MenuItemId {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

impl From<Vec<MenuNode>> for MenuSpec {
    fn from(nodes: Vec<MenuNode>) -> Self {
        Self { nodes }
    }
}
