//! Panel layout and drawing (single menu column).

use super::menu::{MenuIcon, MenuNode};
use super::style::ContextMenuStyle;

use iced::advanced::layout;
use iced::advanced::renderer;
use iced::advanced::svg;
use iced::advanced::text::{self, Paragraph};
use iced::alignment;
use iced::mouse;
use iced::{Color, Font, Pixels, Point, Rectangle, Size};

fn icon_column_width(style: &ContextMenuStyle, icons_enabled: bool) -> f32 {
    if icons_enabled {
        style.icon_slot_width + style.icon_label_gap
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

pub(crate) fn row_geometries<'a>(nodes: &[MenuNode<'a>], style: &ContextMenuStyle) -> Vec<RowGeom> {
    let mut out = Vec::new();
    let mut y = 0.0_f32;
    for (node_idx, node) in nodes.iter().enumerate() {
        let h = match node {
            MenuNode::Separator => style.separator_margin_vertical * 2.0 + style.separator_height,
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

/// Hit-test using the same row bands as layout (excludes `row_spacing` gaps between rows).
/// `panel_relative_y` is the cursor Y in coordinates of the panel node's [`Layout::bounds`].
pub(crate) fn row_index_at_panel_y<'a>(
    nodes: &[MenuNode<'a>],
    style: &ContextMenuStyle,
    panel_relative_y: f32,
) -> Option<usize> {
    let y_content = panel_relative_y - style.border_width - style.panel_padding;
    if y_content < 0.0 {
        return None;
    }
    let geoms = row_geometries(nodes, style);
    for g in &geoms {
        let row_h = if matches!(nodes[g.node_idx], MenuNode::Separator) {
            g.height
        } else {
            g.height - style.row_spacing
        };
        if y_content >= g.y_offset && y_content < g.y_offset + row_h {
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

fn measure_hotkey_width<Renderer: text::Renderer>(
    renderer: &Renderer,
    style: &ContextMenuStyle,
    hotkey: &str,
) -> f32 {
    let size = Pixels(style.hotkey_label_size);
    let line_height = text::LineHeight::default();
    let text = text::Text {
        content: hotkey,
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

fn max_hotkey_width_in_panel<'a, Renderer: text::Renderer>(
    renderer: &Renderer,
    style: &ContextMenuStyle,
    nodes: &[MenuNode<'a>],
) -> f32 {
    let mut m = 0.0_f32;
    for node in nodes {
        if let MenuNode::Action {
            hotkey: Some(h), ..
        } = node
        {
            m = m.max(measure_hotkey_width(renderer, style, h.as_ref()));
        }
    }
    m
}

fn panel_content_width<'a, Renderer: text::Renderer>(
    renderer: &Renderer,
    style: &ContextMenuStyle,
    nodes: &[MenuNode<'a>],
    icons_enabled: bool,
) -> f32 {
    let icon_extra = icon_column_width(style, icons_enabled);
    let max_hk = max_hotkey_width_in_panel(renderer, style, nodes);
    let hk_strip = if max_hk > 0.0 {
        style.label_hotkey_gap + max_hk
    } else {
        0.0
    };
    let mut w = style.min_width;
    let h_margin = style.panel_padding + style.row_label_inset;
    for node in nodes {
        let label = match node {
            MenuNode::Action { title, .. } => title.as_ref(),
            MenuNode::Submenu { title, .. } => title.as_ref(),
            MenuNode::Separator => continue,
        };
        let lw = measure_label_width(renderer, style, label);
        let row_need = match node {
            MenuNode::Action { .. } => lw + h_margin * 2.0 + icon_extra + hk_strip,
            MenuNode::Submenu { .. } => {
                lw + h_margin * 2.0 + icon_extra + style.submenu_chevron_slot_width
            }
            MenuNode::Separator => continue,
        };
        w = w.max(row_need);
    }
    w
}

pub(crate) fn layout_panel<'a, Renderer: text::Renderer>(
    renderer: &Renderer,
    style: &ContextMenuStyle,
    nodes: &[MenuNode<'a>],
    anchor: Point,
    viewport: Size,
    icons_enabled: bool,
    submenu_horizontal_overlap: f32,
) -> (layout::Node, f32, f32) {
    let width = panel_content_width(renderer, style, nodes, icons_enabled);
    let geoms = row_geometries(nodes, style);
    let content_h = panel_height(&geoms);
    let border = style.border_width * 2.0;
    let panel_w = width + border;
    let panel_h = content_h + border + style.panel_padding * 2.0;

    let space_right = viewport.width - anchor.x;
    let space_left = anchor.x;
    let place_right = space_right >= panel_w || space_right >= space_left;
    let mut x = if place_right {
        anchor.x
    } else {
        anchor.x - panel_w
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

fn draw_row_icon<Renderer>(renderer: &mut Renderer, style: &ContextMenuStyle, icon: &MenuIcon, row_bounds: Rectangle, slot_left_x: f32, clip_bounds: Rectangle, color: Color)
where
    Renderer: text::Renderer<Font = Font> + svg::Renderer,
{
    match icon {
        MenuIcon::Svg(handle) => {
            let natural = renderer.measure_svg(handle);
            let nw = natural.width.max(1) as f32;
            let nh = natural.height.max(1) as f32;
            let slot = style.icon_slot_width;
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
                    bounds: Size::new(style.icon_slot_width, row_bounds.height),
                    size: Pixels(style.icon_glyph_size),
                    line_height: text::LineHeight::default(),
                    font,
                    align_x: text::Alignment::Center,
                    align_y: alignment::Vertical::Center,
                    shaping: *shaping,
                    wrapping: text::Wrapping::None,
                },
                Point::new(
                    slot_left_x + style.icon_slot_width * 0.5,
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
            border: style.panel_border(),
            shadow: style.panel_shadow,
            ..renderer::Quad::default()
        },
        style.panel_background,
    );

    let geoms = row_geometries(nodes, style);
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

    let text_size = Pixels(style.label_size);
    let line_height = text::LineHeight::default();
    let font = renderer.default_font();
    let icon_col = icon_column_width(style, icons_enabled);
    let max_hk = max_hotkey_width_in_panel(renderer, style, nodes);
    let row_content_left =
        |row_bounds: Rectangle| row_bounds.x + style.panel_padding + style.row_label_inset;
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

        if show_row_highlight {
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
                    border: style.row_highlight_border(),
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
                icon,
                hotkey,
                ..
            } => {
                let color = if *enabled {
                    style.label_color
                } else {
                    style.disabled_color
                };
                if icons_enabled {
                    if let Some(ic) = icon {
                        draw_row_icon(
                            renderer,
                            style,
                            ic,
                            row_bounds,
                            row_content_left(row_bounds),
                            clip_bounds,
                            color,
                        );
                    }
                }
                let label_x = label_x_for_row(row_bounds);
                let content_right =
                    row_bounds.x + row_bounds.width - style.panel_padding - style.row_label_inset;
                let label_bounds_w = if max_hk > 0.0 {
                    (content_right - max_hk - style.label_hotkey_gap - label_x).max(0.0)
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
                        let hk_color = if *enabled {
                            style.hotkey_label_color
                        } else {
                            style.disabled_color
                        };
                        let hk_size = Pixels(style.hotkey_label_size);
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
                if icons_enabled {
                    if let Some(ic) = icon {
                        draw_row_icon(
                            renderer,
                            style,
                            ic,
                            row_bounds,
                            row_content_left(row_bounds),
                            clip_bounds,
                            style.label_color,
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
                    style.label_color,
                    clip_bounds,
                );
                let handle = style.submenu_chevron_icon.handle();
                let natural = renderer.measure_svg(&handle);
                let nw = natural.width.max(1) as f32;
                let nh = natural.height.max(1) as f32;
                let slot = style.submenu_chevron_slot_width;
                let max_w = slot;
                let max_h = row_bounds.height * 0.92;
                let scale = (max_w / nw).min(max_h / nh);
                let w = nw * scale;
                let h = nh * scale;
                let column_left = row_bounds.x + row_bounds.width
                    - style.panel_padding
                    - style.row_label_inset
                    - slot;
                let svg_bounds = Rectangle {
                    x: column_left + (slot - w) * 0.5,
                    y: row_bounds.y + (row_bounds.height - h) * 0.5,
                    width: w,
                    height: h,
                };
                renderer.draw_svg(
                    svg::Svg::new(handle).color(style.label_color),
                    svg_bounds,
                    clip_bounds,
                );
            }
        }
    }

    let _ = depth;
}
