//! Visual defaults for context menu panels and rows.

use crate::SubmenuChevronIcon;

use iced::border::Radius;
use iced::theme::palette;
use iced::{Border, Color, Shadow, Theme, Vector};

/// Colors and measurements for [`ContextMenu`](crate::ContextMenu) overlays.
#[derive(Debug, Clone)]
pub struct ContextMenuStyle {
    /// Panel fill behind menu items.
    pub panel_background: Color,
    pub panel_border: Color,
    pub border_width: f32,
    pub border_radius: f32,
    /// Padding inside the framed panel.
    pub panel_padding: f32,
    /// Extra horizontal inset for row labels (and chevron) inside the hover margin.
    pub row_label_inset: f32,
    pub min_width: f32,
    pub row_spacing: f32,
    pub label_size: f32,
    /// Vector icon at the end of submenu rows (`svg/` assets).
    pub submenu_chevron_icon: SubmenuChevronIcon,
    /// Horizontal space reserved for the submenu chevron column.
    pub submenu_chevron_slot_width: f32,
    /// Horizontal overlap of nested submenu flyouts with the parent panel (`0` = flush).
    pub submenu_flyout_overlap: f32,
    /// Width reserved for optional row icons (when [`crate::ContextMenu::show_item_icons`] is true).
    pub icon_slot_width: f32,
    /// Space between the icon column and the label.
    pub icon_label_gap: f32,
    /// Font size for optional action hotkey hints on the right.
    pub hotkey_label_size: f32,
    /// Space between the title and the hotkey column (when any action has a hotkey).
    pub label_hotkey_gap: f32,
    /// Color for hotkey hints when the action is enabled (typically more muted than [`Self::label_color`]).
    pub hotkey_label_color: Color,
    pub label_color: Color,
    pub disabled_color: Color,
    pub separator_color: Color,
    pub separator_height: f32,
    pub separator_margin_vertical: f32,
    /// Estimated row height for viewport clamping.
    pub row_height: f32,
    /// Action / submenu row background while hovered.
    pub row_hover_background: Color,
    /// Action / submenu row background while pressed.
    pub row_pressed_background: Color,
    /// Drop shadow under menu panels (root and flyouts).
    pub panel_shadow: Shadow,
    /// Dimmed scrim over content (dismiss layer). Use alpha 0 for invisible.
    pub dismiss_scrim: Color,
}

fn default_panel_shadow() -> Shadow {
    Shadow {
        color: Color::from_rgba(0.0, 0.0, 0.0, 0.35),
        offset: Vector::new(4.0, 4.0),
        blur_radius: 12.0,
    }
}

impl Default for ContextMenuStyle {
    fn default() -> Self {
        Self {
            panel_background: Color::from_rgb(0.14, 0.14, 0.16),
            panel_border: Color::from_rgb(0.32, 0.32, 0.36),
            border_width: 1.0,
            border_radius: 6.0,
            panel_padding: 6.0,
            row_label_inset: 6.0,
            min_width: 160.0,
            row_spacing: 2.0,
            label_size: 14.0,
            submenu_chevron_icon: SubmenuChevronIcon::default(),
            submenu_chevron_slot_width: 20.0,
            submenu_flyout_overlap: 5.0,
            icon_slot_width: 18.0,
            icon_label_gap: 6.0,
            hotkey_label_size: 12.0,
            label_hotkey_gap: 14.0,
            hotkey_label_color: Color::from_rgb(0.62, 0.62, 0.68),
            label_color: Color::from_rgb(0.92, 0.92, 0.94),
            disabled_color: Color::from_rgb(0.45, 0.45, 0.5),
            separator_color: Color::from_rgb(0.35, 0.35, 0.4),
            separator_height: 1.0,
            separator_margin_vertical: 6.0,
            row_height: 28.0,
            row_hover_background: Color::from_rgb(0.32, 0.34, 0.40),
            row_pressed_background: Color::from_rgb(0.24, 0.26, 0.32),
            panel_shadow: default_panel_shadow(),
            dismiss_scrim: Color::from_rgba(0.0, 0.0, 0.0, 0.15),
        }
    }
}

impl ContextMenuStyle {
    /// Menu colors derived from an [`iced::Theme`] palette (spacing and sizes match [`Self::default`]).
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
        style.hotkey_label_color =
            palette::mix(surface.text, surface.color, 0.45);
        // `secondary.weak.text` is tuned for that swatch’s background, not the menu surface, so on
        // light themes it can disappear over `surface`. Blend toward the panel instead.
        style.disabled_color = palette::mix(surface.text, surface.color, 0.56);
        style.separator_color = e.background.strong.color;
        style.row_hover_background = e.background.neutral.color;
        style.row_pressed_background = e.background.stronger.color;

        style.dismiss_scrim = Color::from_rgba(
            0.0,
            0.0,
            0.0,
            if e.is_dark { 0.18 } else { 0.12 },
        );

        let shadow_alpha = if e.is_dark { 0.35 } else { 0.22 };
        style.panel_shadow = Shadow {
            color: Color::from_rgba(0.0, 0.0, 0.0, shadow_alpha),
            offset: Vector::new(4.0, 4.0),
            blur_radius: style.panel_shadow.blur_radius,
        };

        style
    }

    /// Same colors as [`Theme::Dark`] via [`Self::from_theme`]. Differs from [`Self::example_dark`], which uses a fixed demo palette.
    pub fn dark() -> Self {
        Self::from_theme(&Theme::Dark)
    }

    /// Same colors as [`Theme::Light`] via [`Self::from_theme`]. Differs from [`Self::example_light`], which uses a fixed demo palette.
    pub fn light() -> Self {
        Self::from_theme(&Theme::Light)
    }

    pub(crate) fn panel_border(&self) -> Border {
        Border {
            width: self.border_width,
            color: self.panel_border,
            radius: Radius::from(self.border_radius),
        }
    }

    /// Border shape for row hover / pressed highlights (radius matches the panel).
    pub(crate) fn row_highlight_border(&self) -> Border {
        Border {
            width: 0.0,
            color: Color::TRANSPARENT,
            radius: Radius::from(self.border_radius),
        }
    }
}
