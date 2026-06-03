//! Visual defaults for context menu panels and rows.

use iced::border::Radius;
use iced::theme::palette;
use iced::{Border, Color, Shadow, Theme, Vector};

/// Colors and effects for [`ContextMenu`](crate::ContextMenu) overlays.
///
/// Spacing, sizing, and typography measurements are configured on [`ContextMenu`](crate::ContextMenu)
/// via builder methods (e.g. [`ContextMenu::panel_padding`](crate::ContextMenu::panel_padding)).
#[derive(Debug, Clone)]
pub struct ContextMenuStyle {
    /// Panel fill behind menu items.
    pub panel_background: Color,
    pub panel_border: Color,
    pub label_color: Color,
    pub disabled_color: Color,
    /// Color for hotkey hints when the action is enabled (typically more muted than [`Self::label_color`]).
    pub hotkey_label_color: Color,
    pub separator_color: Color,
    /// Action / submenu row background while hovered.
    pub row_hover_background: Color,
    /// Label and icon color on rows with a hover / focus / open highlight.
    pub row_hover_label_color: Color,
    /// Hotkey hint color on highlighted enabled rows (typically muted vs [`Self::row_hover_label_color`]).
    pub row_hover_hotkey_label_color: Color,
    /// Action / submenu row background while pressed.
    pub row_pressed_background: Color,
    /// Drop shadow under menu panels (root and flyouts).
    pub panel_shadow: Shadow,
    /// Dimmed scrim over content (dismiss layer). Use alpha 0 for invisible.
    pub dismiss_scrim: Color,
}

impl Default for ContextMenuStyle {
    fn default() -> Self {
        Self {
            panel_background: Color::from_rgb(0.14, 0.14, 0.16),
            panel_border: Color::from_rgb(0.32, 0.32, 0.36),
            label_color: Color::from_rgb(0.92, 0.92, 0.94),
            disabled_color: Color::from_rgb(0.45, 0.45, 0.5),
            hotkey_label_color: Color::from_rgb(0.62, 0.62, 0.68),
            separator_color: Color::from_rgb(0.35, 0.35, 0.4),
            row_hover_background: Color::from_rgb(0.32, 0.34, 0.40),
            row_hover_label_color: Color::from_rgb(0.98, 0.98, 1.0),
            row_hover_hotkey_label_color: Color::from_rgb(0.75, 0.76, 0.82),
            row_pressed_background: Color::from_rgb(0.24, 0.26, 0.32),
            panel_shadow: Shadow {
                color: Color::from_rgba(0.0, 0.0, 0.0, 0.35),
                offset: Vector::new(4.0, 4.0),
                blur_radius: 12.0,
            },
            dismiss_scrim: Color::from_rgba(0.0, 0.0, 0.0, 0.15),
        }
    }
}

impl ContextMenuStyle {
    /// Menu colors derived from an [`iced::Theme`] palette.
    ///
    /// Use this when the menu should match the application theme, including named variants
    /// (Tokyo Night, Catppuccin, etc.), not only built-in dark/light.
    pub fn from_theme(theme: &Theme) -> Self {
        let mut style = Self::default();
        let e = theme.extended_palette();
        let surface = e.background.weak;

        style.panel_background = surface.color;
        style.panel_border = e.background.strong.color;
        style.label_color = surface.text;
        style.hotkey_label_color = palette::mix(surface.text, surface.color, 0.45);
        // `secondary.weak.text` is tuned for that swatch’s background, not the menu surface, so on
        // light themes it can disappear over `surface`. Blend toward the panel instead.
        style.disabled_color = palette::mix(surface.text, surface.color, 0.56);
        style.separator_color = e.background.strong.color;
        let neutral = e.background.neutral;
        style.row_hover_background = neutral.color;
        style.row_hover_label_color = neutral.text;
        style.row_hover_hotkey_label_color =
            palette::mix(neutral.text, neutral.color, 0.45);
        style.row_pressed_background = e.background.stronger.color;

        style.dismiss_scrim = Color::from_rgba(0.0, 0.0, 0.0, if e.is_dark { 0.18 } else { 0.12 });

        let shadow_alpha = if e.is_dark { 0.35 } else { 0.22 };
        style.panel_shadow = Shadow {
            color: Color::from_rgba(0.0, 0.0, 0.0, shadow_alpha),
            offset: Vector::new(4.0, 4.0),
            blur_radius: style.panel_shadow.blur_radius,
        };

        style
    }

    /// Same colors as [`Theme::Dark`] via [`Self::from_theme`].
    pub fn dark() -> Self {
        Self::from_theme(&Theme::Dark)
    }

    /// Same colors as [`Theme::Light`] via [`Self::from_theme`].
    pub fn light() -> Self {
        Self::from_theme(&Theme::Light)
    }

    pub(crate) fn panel_border(&self, border_width: f32, border_radius: f32) -> Border {
        Border {
            width: border_width,
            color: self.panel_border,
            radius: Radius::from(border_radius),
        }
    }

    /// Border shape for row hover / pressed highlights (radius matches the panel).
    pub(crate) fn row_highlight_border(&self, border_radius: f32) -> Border {
        Border {
            width: 0.0,
            color: Color::TRANSPARENT,
            radius: Radius::from(border_radius),
        }
    }
}

/// The theme catalog of a [`ContextMenu`](crate::ContextMenu).
///
/// All themes that can be used with [`ContextMenu`] must implement this trait.
pub trait Catalog {
    /// The item class of the [`Catalog`].
    type Class<'a>;

    /// The default class produced by the [`Catalog`].
    fn default<'a>() -> Self::Class<'a>;

    /// The [`ContextMenuStyle`] of a class.
    fn style(&self, class: &Self::Class<'_>) -> ContextMenuStyle;
}

/// A styling function for a [`ContextMenu`](crate::ContextMenu).
pub type StyleFn<'a, Theme> = Box<dyn Fn(&Theme) -> ContextMenuStyle + 'a>;

impl Catalog for Theme {
    type Class<'a> = StyleFn<'a, Self>;

    fn default<'a>() -> Self::Class<'a> {
        Box::new(ContextMenuStyle::from_theme)
    }

    fn style(&self, class: &Self::Class<'_>) -> ContextMenuStyle {
        class(self)
    }
}

/// Palette-aligned menu colors; use as `.style(ContextMenuStyle::themed)` on [`iced::Theme`].
pub fn themed(theme: &Theme) -> ContextMenuStyle {
    ContextMenuStyle::from_theme(theme)
}
