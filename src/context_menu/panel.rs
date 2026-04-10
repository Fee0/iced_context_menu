//! Panel layout and drawing (single menu column).

use super::menu::MenuNode;
use super::style::ContextMenuStyle;

use iced::advanced::layout;
use iced::advanced::renderer;
use iced::advanced::text::{self, Paragraph};
use iced::alignment;
use iced::border;
use iced::mouse;
use iced::{Pixels, Point, Rectangle, Size};

pub(crate) const SUBMENU_CHEVRON: &str = "›";
/// Reserved width for the submenu glyph (padding is separate in `panel_content_width`).
pub(crate) const SUBMENU_CHEVRON_WIDTH: f32 = 12.0;

pub(crate) type Layout<'a> = iced::advanced::Layout<'a>;

pub(crate) struct RowGeom {
    pub y_offset: f32,
    pub height: f32,
    pub node_idx: usize,
}

pub(crate) fn row_geometries(nodes: &[MenuNode], style: &ContextMenuStyle) -> Vec<RowGeom> {
    let mut out = Vec::new();
    let mut y = 0.0_f32;
    for (node_idx, node) in nodes.iter().enumerate() {
        let h = match node {
            MenuNode::Separator => {
                style.separator_margin_vertical * 2.0 + style.separator_height
            }
            _ => style.row_height + style.row_spacing,
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

pub(crate) fn row_index_at_y(
    nodes: &[MenuNode],
    style: &ContextMenuStyle,
    y: f32,
) -> Option<usize> {
    let geoms = row_geometries(nodes, style);
    for g in geoms {
        if y >= g.y_offset && y < g.y_offset + g.height {
            return Some(g.node_idx);
        }
    }
    None
}

fn measure_label_width<Renderer: text::Renderer>(
    renderer: &Renderer,
    style: &ContextMenuStyle,
    label: &str,
) -> f32 {
    let size = Pixels(style.label_size);
    let line_height = text::LineHeight::default();
    let text = text::Text {
        content: label,
        bounds: Size::new(f32::INFINITY, style.row_height),
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

fn panel_content_width<Renderer: text::Renderer>(
    renderer: &Renderer,
    style: &ContextMenuStyle,
    nodes: &[MenuNode],
) -> f32 {
    let mut w = style.min_width;
    for node in nodes {
        let label = match node {
            MenuNode::Action { title, .. } => title.as_str(),
            MenuNode::Submenu { title, .. } => title.as_str(),
            MenuNode::Separator => continue,
        };
        let lw = measure_label_width(renderer, style, label);
        let extra = match node {
            MenuNode::Submenu { .. } => SUBMENU_CHEVRON_WIDTH,
            _ => 0.0,
        };
        let h_margin = style.panel_padding + style.row_label_inset;
        w = w.max(lw + h_margin * 2.0 + extra);
    }
    w
}

pub(crate) fn layout_panel<Renderer: text::Renderer>(
    renderer: &Renderer,
    style: &ContextMenuStyle,
    nodes: &[MenuNode],
    anchor: Point,
    viewport: Size,
) -> (layout::Node, f32, f32) {
    let width = panel_content_width(renderer, style, nodes);
    let geoms = row_geometries(nodes, style);
    let content_h = panel_height(&geoms);
    let border = style.border_width * 2.0;
    let panel_w = width + border;
    let panel_h = content_h + border + style.panel_padding * 2.0;

    let space_right = viewport.width - anchor.x;
    let space_left = anchor.x;
    let place_right = space_right >= panel_w || space_right >= space_left;
    let x = if place_right {
        anchor.x
    } else {
        anchor.x - panel_w
    }
    .clamp(0.0, (viewport.width - panel_w).max(0.0));

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
                g.height - if matches!(nodes[g.node_idx], MenuNode::Separator) {
                    0.0
                } else {
                    style.row_spacing
                },
            ))
            .move_to(Point::new(0.0, style.panel_padding + g.y_offset))
        })
        .collect();

    let inner = layout::Node::with_children(
        Size::new(width, content_h + style.panel_padding * 2.0),
        row_nodes,
    )
    .move_to(Point::new(style.border_width, style.border_width));

    let panel = layout::Node::with_children(Size::new(panel_w, panel_h), vec![inner])
        .move_to(Point::new(x, y));

    (panel, panel_w, panel_h)
}

pub(crate) fn draw_panel<Renderer: text::Renderer>(
    renderer: &mut Renderer,
    style: &ContextMenuStyle,
    nodes: &[MenuNode],
    layout: Layout<'_>,
    cursor: mouse::Cursor,
    focus_path: &[usize],
    prefix_path: &[usize],
    clip_bounds: Rectangle,
    depth: usize,
) where
    Renderer: text::Renderer,
{
    let bounds = layout.bounds();
    renderer.fill_quad(
        renderer::Quad {
            bounds,
            border: style.panel_border(),
            shadow: style.panel_shadow(),
            ..renderer::Quad::default()
        },
        style.panel_background,
    );

    let geoms = row_geometries(nodes, style);
    let row_layouts: Vec<_> = layout.children().collect();
    let inner = row_layouts.first();
    let row_lays: Vec<_> = inner
        .map(|l| l.children().collect())
        .unwrap_or_default();

    let text_size = Pixels(style.label_size);
    let line_height = text::LineHeight::default();
    let font = renderer.default_font();

    for g in &geoms {
        let Some(rl) = row_lays.get(g.node_idx) else {
            continue;
        };
        let row_bounds = rl.bounds();
        let node = &nodes[g.node_idx];

        let mut row_path = prefix_path.to_vec();
        row_path.push(g.node_idx);
        let is_focused = focus_path == row_path.as_slice();

        let is_hover = cursor
            .position()
            .is_some_and(|p| row_bounds.contains(p))
            && !matches!(node, MenuNode::Separator);

        let pressed = false;

        if is_focused || is_hover {
            if !matches!(node, MenuNode::Separator) {
                let pad = style.panel_padding;
                let highlight_bounds = Rectangle {
                    x: row_bounds.x + pad,
                    y: row_bounds.y,
                    width: (row_bounds.width - pad * 2.0).max(0.0),
                    height: row_bounds.height,
                };
                renderer.fill_quad(
                    renderer::Quad {
                        bounds: highlight_bounds,
                        border: border::rounded(4.0),
                        ..renderer::Quad::default()
                    },
                    if pressed {
                        style.row_pressed_background
                    } else {
                        style.row_hover_background
                    },
                );
            }
        }

        match node {
            MenuNode::Separator => {
                let y = row_bounds.center_y();
                let h_margin = style.panel_padding + style.row_label_inset;
                renderer.fill_quad(
                    renderer::Quad {
                        bounds: Rectangle {
                            x: row_bounds.x + h_margin,
                            y: y - style.separator_height * 0.5,
                            width: (row_bounds.width - h_margin * 2.0).max(0.0),
                            height: style.separator_height,
                        },
                        ..renderer::Quad::default()
                    },
                    style.separator_color,
                );
            }
            MenuNode::Action {
                title,
                enabled,
                ..
            } => {
                let color = if *enabled {
                    style.label_color
                } else {
                    style.disabled_color
                };
                renderer.fill_text(
                    text::Text {
                        content: title.clone(),
                        bounds: Size::new(f32::INFINITY, row_bounds.height),
                        size: text_size,
                        line_height,
                        font,
                        align_x: text::Alignment::Left,
                        align_y: alignment::Vertical::Center,
                        shaping: text::Shaping::default(),
                        wrapping: text::Wrapping::None,
                    },
                    Point::new(
                        row_bounds.x + style.panel_padding + style.row_label_inset,
                        row_bounds.center_y(),
                    ),
                    color,
                    clip_bounds,
                );
            }
            MenuNode::Submenu { title, .. } => {
                let label_x = row_bounds.x + style.panel_padding + style.row_label_inset;
                renderer.fill_text(
                    text::Text {
                        content: title.clone(),
                        bounds: Size::new(f32::INFINITY, row_bounds.height),
                        size: text_size,
                        line_height,
                        font,
                        align_x: text::Alignment::Left,
                        align_y: alignment::Vertical::Center,
                        shaping: text::Shaping::default(),
                        wrapping: text::Wrapping::None,
                    },
                    Point::new(label_x, row_bounds.center_y()),
                    style.label_color,
                    clip_bounds,
                );
                let chevron_x = row_bounds.x + row_bounds.width
                    - style.panel_padding
                    - style.row_label_inset
                    - SUBMENU_CHEVRON_WIDTH;
                renderer.fill_text(
                    text::Text {
                        content: SUBMENU_CHEVRON.to_string(),
                        bounds: Size::new(SUBMENU_CHEVRON_WIDTH, row_bounds.height),
                        size: text_size,
                        line_height,
                        font,
                        align_x: text::Alignment::Center,
                        align_y: alignment::Vertical::Center,
                        shaping: text::Shaping::default(),
                        wrapping: text::Wrapping::None,
                    },
                    Point::new(chevron_x, row_bounds.center_y()),
                    style.label_color,
                    clip_bounds,
                );
            }
        }
    }

    let _ = depth;
}
