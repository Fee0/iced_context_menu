//! Panel layout and drawing (single menu column).

use super::menu::{MenuIcon, MenuNode};
use super::style::ContextMenuStyle;
use crate::SubmenuChevronIcon;

use iced::advanced::layout;
use iced::advanced::renderer;
use iced::advanced::svg;
use iced::advanced::text::{self, Paragraph};
use iced::alignment;
use iced::mouse;
use iced::{Color, Font, Pixels, Point, Rectangle, Size};

/// Layout measurements copied from [`super::widget::ContextMenu`] for panel code (avoids a widget/panel module cycle).
#[derive(Debug, Clone, Copy)]
pub(crate) struct PanelMetrics {
    pub border_width: f32,
    pub border_radius: f32,
    pub panel_padding: f32,
    pub row_label_inset: f32,
    pub min_width: f32,
    pub row_spacing: f32,
    pub label_size: f32,
    pub submenu_chevron_icon: SubmenuChevronIcon,
    pub submenu_chevron_slot_width: f32,
    pub submenu_flyout_overlap: f32,
    pub icon_slot_width: f32,
    pub icon_label_gap: f32,
    pub icon_glyph_size: f32,
    pub hotkey_label_size: f32,
    pub label_hotkey_gap: f32,
    pub separator_height: f32,
    pub separator_margin_vertical: f32,
    pub row_height: f32,
}

fn icon_column_width(metrics: &PanelMetrics, icons_enabled: bool) -> f32 {
    if icons_enabled {
        metrics.icon_slot_width + metrics.icon_label_gap
    } else {
        0.0
    }
}

pub(crate) type Layout<'a> = iced::advanced::Layout<'a>;

pub(crate) struct RowGeom {
    pub y_offset: f32,
    pub height: f32,
    pub node_idx: usize,
}

pub(crate) fn row_geometries<'a>(nodes: &[MenuNode<'a>], metrics: &PanelMetrics) -> Vec<RowGeom> {
    let mut out = Vec::new();
    let mut y = 0.0_f32;
    for (node_idx, node) in nodes.iter().enumerate() {
        let h = match node {
            MenuNode::Separator => {
                metrics.separator_margin_vertical * 2.0 + metrics.separator_height
            }
            _ => metrics.row_height + metrics.row_spacing,
        };
        out.push(RowGeom {
            y_offset: y,
            height: h,
            node_idx,
        });
        y += h;
    }
    out
}

fn panel_height(geoms: &[RowGeom]) -> f32 {
    geoms.last().map(|g| g.y_offset + g.height).unwrap_or(0.0)
}

/// Hit-test using the same row bands as layout (excludes `row_spacing` gaps between rows).
/// `panel_relative_y` is the cursor Y in coordinates of the panel node's [`Layout::bounds`].
pub(crate) fn row_index_at_panel_y<'a>(
    nodes: &[MenuNode<'a>],
    metrics: &PanelMetrics,
    panel_relative_y: f32,
) -> Option<usize> {
    let y_content = panel_relative_y - metrics.border_width - metrics.panel_padding;
    if y_content < 0.0 {
        return None;
    }
    let geoms = row_geometries(nodes, metrics);
    for g in &geoms {
        let row_h = if matches!(nodes[g.node_idx], MenuNode::Separator) {
            g.height
        } else {
            g.height - metrics.row_spacing
        };
        if y_content >= g.y_offset && y_content < g.y_offset + row_h {
            return Some(g.node_idx);
        }
    }
    None
}

fn measure_label_width<Renderer: text::Renderer>(
    renderer: &Renderer,
    metrics: &PanelMetrics,
    label: &str,
) -> f32 {
    let size = Pixels(metrics.label_size);
    let line_height = text::LineHeight::default();
    let text = text::Text {
        content: label,
        bounds: Size::new(f32::INFINITY, metrics.row_height),
        size,
        line_height,
        font: renderer.default_font(),
        align_x: text::Alignment::Left,
        align_y: alignment::Vertical::Top,
        shaping: text::Shaping::default(),
        wrapping: text::Wrapping::None,
    };
    <<Renderer as text::Renderer>::Paragraph as Paragraph>::with_text(text).min_width()
}

fn measure_hotkey_width<Renderer: text::Renderer>(
    renderer: &Renderer,
    metrics: &PanelMetrics,
    hotkey: &str,
) -> f32 {
    let size = Pixels(metrics.hotkey_label_size);
    let line_height = text::LineHeight::default();
    let text = text::Text {
        content: hotkey,
        bounds: Size::new(f32::INFINITY, metrics.row_height),
        size,
        line_height,
        font: renderer.default_font(),
        align_x: text::Alignment::Left,
        align_y: alignment::Vertical::Top,
        shaping: text::Shaping::default(),
        wrapping: text::Wrapping::None,
    };
    <<Renderer as text::Renderer>::Paragraph as Paragraph>::with_text(text).min_width()
}

fn max_hotkey_width_in_panel<'a, Renderer: text::Renderer>(
    renderer: &Renderer,
    metrics: &PanelMetrics,
    nodes: &[MenuNode<'a>],
) -> f32 {
    let mut m = 0.0_f32;
    for node in nodes {
        if let MenuNode::Action {
            hotkey: Some(h), ..
        } = node
        {
            m = m.max(measure_hotkey_width(renderer, metrics, h.as_ref()));
        }
    }
    m
}

fn panel_content_width<'a, Renderer: text::Renderer>(
    renderer: &Renderer,
    metrics: &PanelMetrics,
    nodes: &[MenuNode<'a>],
    icons_enabled: bool,
) -> f32 {
    let icon_extra = icon_column_width(metrics, icons_enabled);
    let max_hk = max_hotkey_width_in_panel(renderer, metrics, nodes);
    let hk_strip = if max_hk > 0.0 {
        metrics.label_hotkey_gap + max_hk
    } else {
        0.0
    };
    let mut w = metrics.min_width;
    let h_margin = metrics.panel_padding + metrics.row_label_inset;
    for node in nodes {
        let label = match node {
            MenuNode::Action { title, .. } => title.as_ref(),
            MenuNode::Submenu { title, .. } => title.as_ref(),
            MenuNode::Separator => continue,
        };
        let lw = measure_label_width(renderer, metrics, label);
        let row_need = match node {
            MenuNode::Action { .. } => lw + h_margin * 2.0 + icon_extra + hk_strip,
            MenuNode::Submenu { .. } => {
                lw + h_margin * 2.0 + icon_extra + metrics.submenu_chevron_slot_width
            }
            MenuNode::Separator => continue,
        };
        w = w.max(row_need);
    }
    w
}

pub(crate) fn layout_panel<'a, Renderer: text::Renderer>(
    renderer: &Renderer,
    metrics: &PanelMetrics,
    nodes: &[MenuNode<'a>],
    anchor: Point,
    parent_left_x: Option<f32>,
    viewport: Size,
    icons_enabled: bool,
    submenu_horizontal_overlap: f32,
) -> (layout::Node, f32, f32) {
    let width = panel_content_width(renderer, metrics, nodes, icons_enabled);
    let geoms = row_geometries(nodes, metrics);
    let content_h = panel_height(&geoms);
    let border = metrics.border_width * 2.0;
    let panel_w = width + border;
    let panel_h = content_h + border + metrics.panel_padding * 2.0;

    let space_right = viewport.width - anchor.x;
    let space_left = anchor.x;
    let place_right = space_right >= panel_w || space_right >= space_left;
    let mut x = if place_right {
        anchor.x
    } else {
        parent_left_x.unwrap_or(anchor.x) - panel_w
    };
    if submenu_horizontal_overlap > 0.0 {
        if place_right {
            x -= submenu_horizontal_overlap;
        } else {
            x += submenu_horizontal_overlap;
        }
    }
    x = x.clamp(0.0, (viewport.width - panel_w).max(0.0));

    let space_below = viewport.height - anchor.y;
    let space_above = anchor.y;
    let y = if space_below >= panel_h || space_below >= space_above {
        anchor.y.clamp(0.0, (viewport.height - panel_h).max(0.0))
    } else {
        (anchor.y - panel_h).clamp(0.0, (viewport.height - panel_h).max(0.0))
    };

    let row_nodes: Vec<layout::Node> = geoms
        .iter()
        .map(|g| {
            layout::Node::new(Size::new(
                width,
                g.height
                    - if matches!(nodes[g.node_idx], MenuNode::Separator) {
                        0.0
                    } else {
                        metrics.row_spacing
                    },
            ))
            .move_to(Point::new(0.0, metrics.panel_padding + g.y_offset))
        })
        .collect();

    let inner = layout::Node::with_children(
        Size::new(width, content_h + metrics.panel_padding * 2.0),
        row_nodes,
    )
    .move_to(Point::new(metrics.border_width, metrics.border_width));

    let panel = layout::Node::with_children(Size::new(panel_w, panel_h), vec![inner])
        .move_to(Point::new(x, y));

    (panel, panel_w, panel_h)
}

fn draw_row_icon<Renderer>(
    renderer: &mut Renderer,
    metrics: &PanelMetrics,
    icon: &MenuIcon,
    row_bounds: Rectangle,
    slot_left_x: f32,
    clip_bounds: Rectangle,
    color: Color,
) where
    Renderer: text::Renderer<Font = Font> + svg::Renderer,
{
    match icon {
        MenuIcon::Svg(handle) => {
            let natural = renderer.measure_svg(handle);
            let nw = natural.width.max(1) as f32;
            let nh = natural.height.max(1) as f32;
            let slot = metrics.icon_slot_width;
            let max_w = slot;
            let max_h = row_bounds.height * 0.92;
            let scale = (max_w / nw).min(max_h / nh);
            let w = nw * scale;
            let h = nh * scale;
            let svg_bounds = Rectangle {
                x: slot_left_x + (slot - w) * 0.5,
                y: row_bounds.y + (row_bounds.height - h) * 0.5,
                width: w,
                height: h,
            };
            renderer.draw_svg(
                svg::Svg::new(handle.clone()).color(color),
                svg_bounds,
                clip_bounds,
            );
        }
        MenuIcon::Glyph {
            glyph,
            font,
            shaping,
        } => {
            let font = font.unwrap_or_else(|| renderer.default_font());
            renderer.fill_text(
                text::Text {
                    content: glyph.as_ref().to_string(),
                    bounds: Size::new(metrics.icon_slot_width, row_bounds.height),
                    size: Pixels(metrics.icon_glyph_size),
                    line_height: text::LineHeight::default(),
                    font,
                    align_x: text::Alignment::Center,
                    align_y: alignment::Vertical::Center,
                    shaping: *shaping,
                    wrapping: text::Wrapping::None,
                },
                Point::new(
                    slot_left_x + metrics.icon_slot_width * 0.5,
                    row_bounds.center_y(),
                ),
                color,
                clip_bounds,
            );
        }
    }
}

pub(crate) fn draw_panel<'a, Renderer>(
    renderer: &mut Renderer,
    metrics: &PanelMetrics,
    style: &ContextMenuStyle,
    nodes: &[MenuNode<'a>],
    layout: Layout<'_>,
    cursor: mouse::Cursor,
    focus_path: &[usize],
    prefix_path: &[usize],
    open_path: &[usize],
    clip_bounds: Rectangle,
    depth: usize,
    icons_enabled: bool,
) where
    Renderer: text::Renderer<Font = Font> + svg::Renderer,
{
    let bounds = layout.bounds();
    renderer.fill_quad(
        renderer::Quad {
            bounds,
            border: style.panel_border(metrics.border_width, metrics.border_radius),
            shadow: style.panel_shadow,
            ..renderer::Quad::default()
        },
        style.panel_background,
    );

    let geoms = row_geometries(nodes, metrics);
    let row_layouts: Vec<_> = layout.children().collect();
    let inner = row_layouts.first();
    let row_lays: Vec<_> = inner.map(|l| l.children().collect()).unwrap_or_default();

    let pointer_row = cursor.position().and_then(|p| {
        geoms.iter().find_map(|g| {
            let rl = row_lays.get(g.node_idx)?;
            let b = rl.bounds();
            if b.contains(p) && !matches!(nodes[g.node_idx], MenuNode::Separator) {
                Some(g.node_idx)
            } else {
                None
            }
        })
    });

    let text_size = Pixels(metrics.label_size);
    let line_height = text::LineHeight::default();
    let font = renderer.default_font();
    let icon_col = icon_column_width(metrics, icons_enabled);
    let max_hk = max_hotkey_width_in_panel(renderer, metrics, nodes);
    let row_content_left =
        |row_bounds: Rectangle| row_bounds.x + metrics.panel_padding + metrics.row_label_inset;
    let label_x_for_row = |row_bounds: Rectangle| row_content_left(row_bounds) + icon_col;

    for g in &geoms {
        let Some(rl) = row_lays.get(g.node_idx) else {
            continue;
        };
        let row_bounds = rl.bounds();
        let node = &nodes[g.node_idx];

        let mut row_path = prefix_path.to_vec();
        row_path.push(g.node_idx);
        let is_focused = focus_path == row_path.as_slice();

        let hovered = pointer_row == Some(g.node_idx);
        let open_chain = !open_path.is_empty() && open_path.starts_with(row_path.as_slice());
        let show_row_highlight = !matches!(node, MenuNode::Separator)
            && (hovered || open_chain || (pointer_row.is_none() && is_focused));

        let pressed = false;
        let row_label_color = |enabled: bool| {
            if !enabled {
                style.disabled_color
            } else if show_row_highlight {
                style.row_hover_label_color
            } else {
                style.label_color
            }
        };
        let row_hotkey_color = |enabled: bool| {
            if !enabled {
                style.disabled_color
            } else if show_row_highlight {
                style.row_hover_hotkey_label_color
            } else {
                style.hotkey_label_color
            }
        };

        if show_row_highlight {
            let pad = metrics.panel_padding;
            let highlight_bounds = Rectangle {
                x: row_bounds.x + pad,
                y: row_bounds.y,
                width: (row_bounds.width - pad * 2.0).max(0.0),
                height: row_bounds.height,
            };
            renderer.fill_quad(
                renderer::Quad {
                    bounds: highlight_bounds,
                    border: style.row_highlight_border(metrics.border_radius),
                    ..renderer::Quad::default()
                },
                if pressed {
                    style.row_pressed_background
                } else {
                    style.row_hover_background
                },
            );
        }

        match node {
            MenuNode::Separator => {
                let y = row_bounds.center_y();
                let h_margin = metrics.panel_padding + metrics.row_label_inset;
                renderer.fill_quad(
                    renderer::Quad {
                        bounds: Rectangle {
                            x: row_bounds.x + h_margin,
                            y: y - metrics.separator_height * 0.5,
                            width: (row_bounds.width - h_margin * 2.0).max(0.0),
                            height: metrics.separator_height,
                        },
                        ..renderer::Quad::default()
                    },
                    style.separator_color,
                );
            }
            MenuNode::Action {
                title,
                enabled,
                icon,
                hotkey,
                ..
            } => {
                let color = row_label_color(*enabled);
                if icons_enabled {
                    if let Some(ic) = icon {
                        draw_row_icon(
                            renderer,
                            metrics,
                            ic,
                            row_bounds,
                            row_content_left(row_bounds),
                            clip_bounds,
                            color,
                        );
                    }
                }
                let label_x = label_x_for_row(row_bounds);
                let content_right = row_bounds.x + row_bounds.width
                    - metrics.panel_padding
                    - metrics.row_label_inset;
                let label_bounds_w = if max_hk > 0.0 {
                    (content_right - max_hk - metrics.label_hotkey_gap - label_x).max(0.0)
                } else {
                    f32::INFINITY
                };
                renderer.fill_text(
                    text::Text {
                        content: title.as_ref().to_string(),
                        bounds: Size::new(label_bounds_w, row_bounds.height),
                        size: text_size,
                        line_height,
                        font,
                        align_x: text::Alignment::Left,
                        align_y: alignment::Vertical::Center,
                        shaping: text::Shaping::default(),
                        wrapping: text::Wrapping::None,
                    },
                    Point::new(label_x, row_bounds.center_y()),
                    color,
                    clip_bounds,
                );
                if max_hk > 0.0 {
                    if let Some(hk) = hotkey.as_ref() {
                        let hk_color = row_hotkey_color(*enabled);
                        let hk_size = Pixels(metrics.hotkey_label_size);
                        let hk_line_height = text::LineHeight::default();
                        renderer.fill_text(
                            text::Text {
                                content: hk.as_ref().to_string(),
                                bounds: Size::new(max_hk, row_bounds.height),
                                size: hk_size,
                                line_height: hk_line_height,
                                font,
                                align_x: text::Alignment::Right,
                                align_y: alignment::Vertical::Center,
                                shaping: text::Shaping::default(),
                                wrapping: text::Wrapping::None,
                            },
                            Point::new(content_right, row_bounds.center_y()),
                            hk_color,
                            clip_bounds,
                        );
                    }
                }
            }
            MenuNode::Submenu { title, icon, .. } => {
                // Match line box to row height so label and larger chevron share the same vertical
                // center as each other and the hover strip (default Relative line height differs per size).
                let row_line_height = text::LineHeight::Absolute(Pixels(row_bounds.height));
                let label_x = label_x_for_row(row_bounds);
                let color = row_label_color(true);
                if icons_enabled {
                    if let Some(ic) = icon {
                        draw_row_icon(
                            renderer,
                            metrics,
                            ic,
                            row_bounds,
                            row_content_left(row_bounds),
                            clip_bounds,
                            color,
                        );
                    }
                }
                renderer.fill_text(
                    text::Text {
                        content: title.as_ref().to_string(),
                        bounds: Size::new(f32::INFINITY, row_bounds.height),
                        size: text_size,
                        line_height: row_line_height,
                        font,
                        align_x: text::Alignment::Left,
                        align_y: alignment::Vertical::Center,
                        shaping: text::Shaping::default(),
                        wrapping: text::Wrapping::None,
                    },
                    Point::new(label_x, row_bounds.center_y()),
                    color,
                    clip_bounds,
                );
                let handle = metrics.submenu_chevron_icon.handle();
                let natural = renderer.measure_svg(&handle);
                let nw = natural.width.max(1) as f32;
                let nh = natural.height.max(1) as f32;
                let slot = metrics.submenu_chevron_slot_width;
                let max_w = slot;
                let max_h = row_bounds.height * 0.92;
                let scale = (max_w / nw).min(max_h / nh);
                let w = nw * scale;
                let h = nh * scale;
                let column_left = row_bounds.x + row_bounds.width
                    - metrics.panel_padding
                    - metrics.row_label_inset
                    - slot;
                let svg_bounds = Rectangle {
                    x: column_left + (slot - w) * 0.5,
                    y: row_bounds.y + (row_bounds.height - h) * 0.5,
                    width: w,
                    height: h,
                };
                renderer.draw_svg(
                    svg::Svg::new(handle).color(color),
                    svg_bounds,
                    clip_bounds,
                );
            }
        }
    }

    let _ = depth;
}
