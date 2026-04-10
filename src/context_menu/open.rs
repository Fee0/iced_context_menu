use iced::Point;

/// How the context menu widget opens. Set each view (e.g. from app state).
///
/// For [`Self::Programmatic`], set `open: true` for one update (or until the menu opens), then
/// clear it—same pulse pattern as other one-shot UI requests in Iced.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ContextMenuOpen {
    /// Open when the user right-clicks over the widget (default).
    RightClick,
    /// Open only when `open` is true this frame. Right-click does not open the menu.
    ///
    /// Use `anchor: Some(p)` for a fixed position, or `None` to use the cursor when it is over
    /// the widget, otherwise the center of the widget bounds.
    Programmatic { open: bool, anchor: Option<Point> },
}

impl Default for ContextMenuOpen {
    fn default() -> Self {
        Self::RightClick
    }
}
