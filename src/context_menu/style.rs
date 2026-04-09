//! Visual defaults for context menu panels and rows.

use iced::border::Radius;
use iced::{Border, Color, Shadow, Vector};

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
    pub min_width: f32,
    pub row_spacing: f32,
    pub label_size: f32,
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
    /// Dimmed scrim over content (dismiss layer). Use alpha 0 for invisible.
    pub dismiss_scrim: Color,
}

impl Default for ContextMenuStyle {
    fn default() -> Self {
        Self {
            panel_background: Color::from_rgb(0.14, 0.14, 0.16),
            panel_border: Color::from_rgb(0.32, 0.32, 0.36),
            border_width: 1.0,
            border_radius: 6.0,
            panel_padding: 6.0,
            min_width: 160.0,
            row_spacing: 2.0,
            label_size: 14.0,
            label_color: Color::from_rgb(0.92, 0.92, 0.94),
            disabled_color: Color::from_rgb(0.45, 0.45, 0.5),
            separator_color: Color::from_rgb(0.35, 0.35, 0.4),
            separator_height: 1.0,
            separator_margin_vertical: 6.0,
            row_height: 28.0,
            row_hover_background: Color::from_rgb(0.32, 0.34, 0.40),
            row_pressed_background: Color::from_rgb(0.24, 0.26, 0.32),
            dismiss_scrim: Color::from_rgba(0.0, 0.0, 0.0, 0.15),
        }
    }
}

impl ContextMenuStyle {
    pub(crate) fn panel_border(&self) -> Border {
        Border {
            width: self.border_width,
            color: self.panel_border,
            radius: Radius::from(self.border_radius),
        }
    }

    pub(crate) fn panel_shadow(&self) -> Shadow {
        Shadow {
            color: Color::from_rgba(0.0, 0.0, 0.0, 0.35),
            offset: Vector::new(0.0, 4.0),
            blur_radius: 12.0,
        }
    }
}
