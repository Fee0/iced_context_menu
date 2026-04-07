use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MenuItemId(pub u64);

impl fmt::Display for MenuItemId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone)]
pub enum MenuNode {
    Action {
        id: MenuItemId,
        title: String,
        enabled: bool,
    },
    Separator,
    Submenu {
        title: String,
        children: Vec<MenuNode>,
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
        });
        self
    }

    pub fn disabled(mut self, id: impl Into<MenuItemId>, title: impl Into<String>) -> Self {
        self.nodes.push(MenuNode::Action {
            id: id.into(),
            title: title.into(),
            enabled: false,
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
