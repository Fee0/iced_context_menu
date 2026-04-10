//! Bundled SVG assets for submenu row indicators.

use std::borrow::Cow;

use iced::advanced::svg;

/// Which vector icon to draw at the end of a [`MenuNode::Submenu`](crate::MenuNode) row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SubmenuChevronIcon {
    /// `svg/arrow-next-small-svgrepo-com.svg`
    #[default]
    ArrowNextSmall,
    /// `svg/arrow-next-svgrepo-com.svg`
    ArrowNext,
    /// `svg/arrow-right-333-svgrepo-com.svg`
    ArrowRight333,
    /// `svg/arrow-right-336-svgrepo-com.svg`
    ArrowRight336,
}

impl SubmenuChevronIcon {
    /// SVG data for this variant (embedded at compile time).
    pub fn handle(self) -> svg::Handle {
        let bytes: &'static [u8] = match self {
            Self::ArrowNextSmall => {
                include_bytes!("../svg/arrow-next-small-svgrepo-com.svg")
            }
            Self::ArrowNext => include_bytes!("../svg/arrow-next-svgrepo-com.svg"),
            Self::ArrowRight333 => include_bytes!("../svg/arrow-right-333-svgrepo-com.svg"),
            Self::ArrowRight336 => include_bytes!("../svg/arrow-right-336-svgrepo-com.svg"),
        };
        svg::Handle::from_memory(Cow::Borrowed(bytes))
    }
}
