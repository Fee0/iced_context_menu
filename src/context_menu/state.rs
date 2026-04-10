//! Menu tree navigation, open/focus paths, and delayed submenu state.

use super::menu::{MenuNode, MenuSpec};

use iced::advanced::Shell;
use iced::time::{Duration as IcedDuration, Instant};
use iced::Point;

/// How nested submenus open.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SubmenuOpenMode {
    /// Open as soon as the pointer enters the submenu row.
    #[default]
    Hover,
    /// Open after the pointer rests on the row for [`crate::ContextMenu::submenu_hover_delay_ms`].
    HoverDelayed,
    /// Open when the submenu row is clicked.
    Click,
}

/// Persistent state for [`crate::ContextMenu`], stored in the widget [`Tree`](iced::advanced::widget::Tree).
#[derive(Debug, Clone)]
pub struct ContextMenuState {
    pub open: bool,
    pub anchor: Point,
    /// Keyboard / logical focus: indices from root through each nested panel.
    pub focus_path: Vec<usize>,
    /// Open submenu chain: `open_path[0]` is a root row index, etc.
    pub open_path: Vec<usize>,
    /// Pending delayed submenu (`HoverDelayed`).
    pub submenu_delay: Option<(Vec<usize>, Instant)>,
    /// Anchor for flyout at depth `d` (`submenu_anchors[d]` = top-left of that flyout panel).
    pub submenu_anchors: Vec<Point>,
}

impl Default for ContextMenuState {
    fn default() -> Self {
        Self {
            open: false,
            anchor: Point::ORIGIN,
            focus_path: Vec::new(),
            open_path: Vec::new(),
            submenu_delay: None,
            submenu_anchors: Vec::new(),
        }
    }
}

impl ContextMenuState {
    pub(crate) fn reset_interaction(&mut self) {
        self.focus_path.clear();
        self.open_path.clear();
        self.submenu_delay = None;
        self.submenu_anchors.clear();
    }

    pub(crate) fn close(&mut self) {
        self.open = false;
        self.reset_interaction();
    }

    pub(crate) fn ensure_focus(&mut self, nodes: &[MenuNode]) {
        if self.focus_path.is_empty() {
            if let Some(i) = first_focusable(nodes, None) {
                self.focus_path.push(i);
            }
        }
    }
}

pub(crate) fn first_focusable(nodes: &[MenuNode], skip: Option<usize>) -> Option<usize> {
    for (i, n) in nodes.iter().enumerate() {
        if skip == Some(i) {
            continue;
        }
        match n {
            MenuNode::Separator => {}
            MenuNode::Action { enabled: false, .. } => {}
            _ => return Some(i),
        }
    }
    None
}

pub(crate) fn next_focusable(nodes: &[MenuNode], from: usize, dir: isize) -> Option<usize> {
    if nodes.is_empty() {
        return None;
    }
    let len = nodes.len() as isize;
    let mut i = from as isize;
    for _ in 0..nodes.len() {
        i = (i + dir).rem_euclid(len);
        let ui = i as usize;
        match &nodes[ui] {
            MenuNode::Separator => continue,
            MenuNode::Action { enabled: false, .. } => continue,
            _ => return Some(ui),
        }
    }
    None
}

pub(crate) fn submenu_children<'a>(nodes: &'a [MenuNode], path: &[usize]) -> Option<&'a [MenuNode]> {
    let mut current = nodes;
    for (d, &idx) in path.iter().enumerate() {
        let node = current.get(idx)?;
        match node {
            MenuNode::Submenu { children, .. } => {
                if d + 1 == path.len() {
                    return Some(children.as_slice());
                }
                current = children.as_slice();
            }
            _ => return None,
        }
    }
    None
}

pub(crate) fn node_at_path<'a>(nodes: &'a [MenuNode], path: &[usize]) -> Option<&'a MenuNode> {
    let mut current = nodes;
    for (d, &idx) in path.iter().enumerate() {
        let node = current.get(idx)?;
        if d + 1 == path.len() {
            return Some(node);
        }
        match node {
            MenuNode::Submenu { children, .. } => current = children.as_slice(),
            _ => return None,
        }
    }
    None
}

pub(crate) fn current_nodes<'a>(root: &'a [MenuNode], focus_path: &[usize]) -> &'a [MenuNode] {
    if focus_path.len() <= 1 {
        return root;
    }
    submenu_children(root, &focus_path[..focus_path.len() - 1]).unwrap_or(root)
}

pub(crate) fn sync_open_path_for_focus<Message>(
    state: &mut ContextMenuState,
    items: &MenuSpec,
    mode: SubmenuOpenMode,
    hover_delay: IcedDuration,
    focus: &[usize],
    shell: &mut Shell<'_, Message>,
) {
    state.submenu_delay = None;
    if focus.is_empty() {
        state.open_path.clear();
        return;
    }

    let root = items.nodes();
    let mut open = Vec::new();
    let mut cur = root;

    for depth in 0..focus.len() {
        let idx = focus[depth];
        let Some(node) = cur.get(idx) else {
            break;
        };
        let is_last = depth + 1 == focus.len();

        if !is_last {
            if let MenuNode::Submenu { children, .. } = node {
                open.push(idx);
                cur = children;
            } else {
                break;
            }
        } else {
            match (node, mode) {
                (MenuNode::Submenu { .. }, SubmenuOpenMode::Hover) => {
                    open.push(idx);
                }
                (MenuNode::Submenu { .. }, SubmenuOpenMode::HoverDelayed) => {
                    state.submenu_delay = Some((focus.to_vec(), Instant::now()));
                    shell.request_redraw_at(Instant::now() + hover_delay);
                    state.open_path = open;
                    return;
                }
                (MenuNode::Submenu { .. }, SubmenuOpenMode::Click) => {
                    if state.open_path.starts_with(focus) {
                        open.push(idx);
                    }
                }
                _ => {}
            }
        }
    }

    state.open_path = open;
}
