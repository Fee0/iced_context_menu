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

use iced::Font;
use iced::advanced::svg;
use iced::advanced::text::Shaping;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MenuItemId(pub u64);

impl fmt::Display for MenuItemId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Row icon shown to the left of a label when [`crate::ContextMenu::show_item_icons`] is true.
///
/// Use [`MenuIcon::from_svg_bytes`] for vector assets, or [`MenuIcon::from_glyph`] for a text glyph
/// (e.g. icon-font codepoints). Glyph icons use [`crate::ContextMenu::icon_glyph_size`] and inherit
/// row label / disabled color at draw time.
#[derive(Debug, Clone)]
pub enum MenuIcon {
    Svg(svg::Handle),
    Glyph {
        glyph: Cow<'static, str>,
        font: Option<Font>,
        shaping: Shaping,
    },
}

impl MenuIcon {
    /// Build from raw SVG bytes; use `include_bytes!` at the call site for embedded assets.
    pub fn from_svg_bytes(bytes: impl Into<Cow<'static, [u8]>>) -> Self {
        Self::Svg(svg::Handle::from_memory(bytes.into()))
    }

    /// Build from a text glyph. `font: None` uses the renderer default font at draw time.
    ///
    /// Prefer [`Shaping::Advanced`] for non-ASCII or private-use icon-font glyphs.
    pub fn from_glyph(
        glyph: impl Into<Cow<'static, str>>,
        font: Option<Font>,
        shaping: Shaping,
    ) -> Self {
        Self::Glyph {
            glyph: glyph.into(),
            font,
            shaping,
        }
    }
}

/// One stop of a [`MenuNode::Slider`]: the id it emits when the slider comes to rest on it, and
/// the value it is labelled with.
#[derive(Debug, Clone)]
pub struct SliderStop<'a> {
    pub id: MenuItemId,
    pub label: Cow<'a, str>,
}

impl<'a> SliderStop<'a> {
    pub fn new(id: impl Into<MenuItemId>, label: impl Into<Cow<'a, str>>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
        }
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
    /// A row carrying on/off state, drawn with a checkbox in the icon slot (a toggle has no icon
    /// of its own). Selecting it emits `id` like a [`MenuNode::Action`]; the app flips `on` and
    /// rebuilds the spec.
    Toggle {
        id: MenuItemId,
        title: Cow<'a, str>,
        on: bool,
        enabled: bool,
        hotkey: Option<Cow<'a, str>>,
    },
    /// A row holding a slider over a handful of discrete stops: its label above, then the groove
    /// with a dot on it at every stop, and under that the values those stops stand for — as many
    /// as fit without crowding, the two ends always among them. The stop the slider rests on is
    /// the one whose value is spelled out in the row's own label color.
    ///
    /// The slider is monochrome by design — groove, dots and handle are drawn from that label
    /// color, like a [`MenuNode::Toggle`] checkbox — so it keeps its contrast in any theme. The
    /// row takes no hover highlight; instead the handle grows a halo while the pointer is on the
    /// row or dragging it.
    ///
    /// Coming to rest on a stop — by click, drag, or arrow key — emits that stop's id like a
    /// [`MenuNode::Action`], once per stop crossed; the menu stays open so the value can be
    /// dialled in. The app applies the value and rebuilds the spec, so `selected` is where the
    /// handle is drawn next.
    Slider {
        title: Cow<'a, str>,
        stops: Vec<SliderStop<'a>>,
        /// Index into `stops` the handle rests on. Out of range leaves the handle at the near
        /// end and no stop reported as current.
        selected: usize,
        enabled: bool,
        icon: Option<MenuIcon>,
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

    pub fn toggle(
        mut self,
        id: impl Into<MenuItemId>,
        title: impl Into<Cow<'a, str>>,
        on: bool,
        hotkey: Option<Cow<'a, str>>,
    ) -> Self {
        self.nodes.push(MenuNode::Toggle {
            id: id.into(),
            title: title.into(),
            on,
            enabled: true,
            hotkey,
        });
        self
    }

    pub fn toggle_disabled(
        mut self,
        id: impl Into<MenuItemId>,
        title: impl Into<Cow<'a, str>>,
        on: bool,
        hotkey: Option<Cow<'a, str>>,
    ) -> Self {
        self.nodes.push(MenuNode::Toggle {
            id: id.into(),
            title: title.into(),
            on,
            enabled: false,
            hotkey,
        });
        self
    }

    pub fn slider(
        mut self,
        title: impl Into<Cow<'a, str>>,
        stops: impl Into<Vec<SliderStop<'a>>>,
        selected: usize,
        icon: Option<MenuIcon>,
    ) -> Self {
        self.nodes.push(MenuNode::Slider {
            title: title.into(),
            stops: stops.into(),
            selected,
            enabled: true,
            icon,
        });
        self
    }

    pub fn slider_disabled(
        mut self,
        title: impl Into<Cow<'a, str>>,
        stops: impl Into<Vec<SliderStop<'a>>>,
        selected: usize,
        icon: Option<MenuIcon>,
    ) -> Self {
        self.nodes.push(MenuNode::Slider {
            title: title.into(),
            stops: stops.into(),
            selected,
            enabled: false,
            icon,
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
