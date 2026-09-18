//! Panel layout and drawing (single menu column).

use super::menu::{MenuIcon, MenuNode, SliderStop};
use super::style::ContextMenuStyle;
use crate::SubmenuChevronIcon;

use iced::advanced::layout;
use iced::advanced::renderer;
use iced::advanced::svg;
use iced::advanced::text::{self, Paragraph};
use iced::alignment;
use iced::border::Radius;
use iced::mouse;
use iced::{Border, Color, Font, Padding, Pixels, Point, Rectangle, Size};

/// Bundled check mark drawn inside a checked [`MenuNode::Toggle`] box.
fn check_handle() -> svg::Handle {
    svg::Handle::from_memory(std::borrow::Cow::Borrowed(
        include_bytes!("../../svg/check.svg").as_slice(),
    ))
}

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
    pub toggle_box_size: f32,
    pub hotkey_label_size: f32,
    pub label_hotkey_gap: f32,
    pub separator_height: f32,
    pub separator_margin_vertical: f32,
    pub row_height: f32,
    pub slider_row_height: f32,
    pub slider_track_height: f32,
    pub slider_handle_size: f32,
    pub slider_handle_halo: f32,
    pub slider_dot_size: f32,
    pub slider_min_track_width: f32,
    pub slider_label_gap: f32,
    pub number_row_height: f32,
    pub number_input_width: f32,
    pub number_text_size: f32,
    pub number_stepper_width: f32,
    pub number_padding: Padding,
}

/// Toggle checkboxes live in the icon slot, so a panel holding one needs that column even when
/// the widget draws no icons.
fn icons_column_enabled<'a>(nodes: &[MenuNode<'a>], icons_enabled: bool) -> bool {
    icons_enabled || nodes.iter().any(|n| matches!(n, MenuNode::Toggle { .. }))
}

fn icon_column_width(metrics: &PanelMetrics, icons_enabled: bool) -> f32 {
    if icons_enabled {
        metrics.icon_slot_width + metrics.icon_label_gap
    } else {
        0.0
    }
}

/// Width the icon column takes from every row of a panel, checkboxes included.
pub(crate) fn icon_column(
    metrics: &PanelMetrics,
    nodes: &[MenuNode<'_>],
    icons_enabled: bool,
) -> f32 {
    icon_column_width(metrics, icons_column_enabled(nodes, icons_enabled))
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
            // A slider spends one line on its label and value and the next on its track.
            MenuNode::Slider { .. } => metrics.slider_row_height + metrics.row_spacing,
            // A number row gives its field more height than a label needs.
            MenuNode::Number { .. } => metrics.number_row_height + metrics.row_spacing,
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
        }
        | MenuNode::Toggle {
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
    let icon_extra = icon_column_width(metrics, icons_column_enabled(nodes, icons_enabled));
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
            MenuNode::Toggle { title, .. } => title.as_ref(),
            MenuNode::Submenu { title, .. } => title.as_ref(),
            MenuNode::Slider { title, .. } => title.as_ref(),
            MenuNode::Number { title, .. } => title.as_ref(),
            MenuNode::Separator => continue,
        };
        let lw = measure_label_width(renderer, metrics, label);
        let row_need = match node {
            MenuNode::Action { .. } | MenuNode::Toggle { .. } => {
                lw + h_margin * 2.0 + icon_extra + hk_strip
            }
            MenuNode::Submenu { .. } => {
                lw + h_margin * 2.0 + icon_extra + metrics.submenu_chevron_slot_width
            }
            // Every line of a slider has to fit: its label, the groove, and the two ends of the
            // scale under it.
            MenuNode::Slider { stops, .. } => {
                let scale_line = scale_ends_width(renderer, metrics, stops);
                h_margin * 2.0 + icon_extra + lw.max(metrics.slider_min_track_width).max(scale_line)
            }
            // The label and the field sit side by side, so the row needs both plus the gap.
            MenuNode::Number { .. } => {
                lw + metrics.label_hotkey_gap
                    + metrics.number_input_width
                    + h_margin * 2.0
                    + icon_extra
            }
            MenuNode::Separator => continue,
        };
        w = w.max(row_need);
    }
    w
}

/// Width the two ends of a slider's scale need side by side — the labels of the first and last
/// stop, which are the two the scale never drops.
fn scale_ends_width<Renderer: text::Renderer>(
    renderer: &Renderer,
    metrics: &PanelMetrics,
    stops: &[SliderStop<'_>],
) -> f32 {
    let width = |stop: Option<&SliderStop<'_>>| {
        stop.map(|stop| measure_hotkey_width(renderer, metrics, stop.label.as_ref()))
            .unwrap_or(0.0)
    };
    match stops.len() {
        0 => 0.0,
        1 => width(stops.first()),
        _ => width(stops.first()) + metrics.slider_label_gap + width(stops.last()),
    }
}

/// Where a slider row's lines sit inside the row, and where its stops fall along them.
///
/// Below the row's label line the slider gives one line to the groove, with a dot on it at every
/// stop, and the next to the values those stops stand for.
#[derive(Debug, Clone, Copy)]
pub(crate) struct SliderTrack {
    /// Everything below the label line: the band a press grabs the handle in, so the dots and
    /// the scale are as much a target as the groove.
    pub band: Rectangle,
    /// The groove, which runs from the first stop to the last: both ends of the line are a stop,
    /// so no stub of it reaches past one.
    pub groove: Rectangle,
    /// Center of the first stop, at the groove's left end.
    pub first_x: f32,
    /// Center of the last stop, at its right end.
    pub last_x: f32,
    /// Center line of the scale labels.
    pub scale_y: f32,
    /// The row's text column, which the scale labels are kept inside. Wider than the groove by
    /// the half handle the groove is held off each end by.
    pub lane: (f32, f32),
}

/// Where the groove and the scale under it are centered inside the band below the label line.
const GROOVE_LINE: f32 = 0.32;
const SCALE_LINE: f32 = 0.78;

impl SliderTrack {
    /// Center of stop `index` of `count`.
    pub fn stop_x(&self, count: usize, index: usize) -> f32 {
        if count <= 1 {
            return self.first_x;
        }
        let t = (index.min(count - 1)) as f32 / (count - 1) as f32;
        self.first_x + (self.last_x - self.first_x) * t
    }

    /// The stop nearest `x`, which is where a press or a drag puts the handle.
    pub fn stop_at_x(&self, count: usize, x: f32) -> usize {
        if count <= 1 {
            return 0;
        }
        let span = self.last_x - self.first_x;
        if span <= 0.0 {
            return 0;
        }
        let t = ((x - self.first_x) / span).clamp(0.0, 1.0);
        (t * (count - 1) as f32).round() as usize
    }
}

/// The track of the slider in `row_bounds`, laid out the way [`draw_panel`] draws it so a press
/// lands on the stop it points at.
pub(crate) fn slider_track(
    metrics: &PanelMetrics,
    icon_col: f32,
    row_bounds: Rectangle,
) -> SliderTrack {
    let left = row_bounds.x + metrics.panel_padding + metrics.row_label_inset + icon_col;
    let right = (row_bounds.x + row_bounds.width - metrics.panel_padding - metrics.row_label_inset)
        .max(left);

    let title_height = metrics.row_height.min(row_bounds.height);
    let band = Rectangle {
        x: row_bounds.x,
        y: row_bounds.y + title_height,
        width: row_bounds.width,
        height: row_bounds.height - title_height,
    };

    // The handle is centered on the stop it rests on, so the groove is held half a handle off
    // each end of the text column and the handle at either end stays inside it.
    let inset = metrics.slider_handle_size * 0.5;
    let groove_left = left + inset;
    let groove_right = (right - inset).max(groove_left);
    let groove = Rectangle {
        x: groove_left,
        y: band.y + band.height * GROOVE_LINE - metrics.slider_track_height * 0.5,
        width: groove_right - groove_left,
        height: metrics.slider_track_height,
    };

    SliderTrack {
        band,
        groove,
        first_x: groove_left,
        last_x: groove_right,
        scale_y: band.y + band.height * SCALE_LINE,
        lane: (left, right),
    }
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
    anchor_row_height: f32,
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
        (anchor.y + anchor_row_height - panel_h).clamp(0.0, (viewport.height - panel_h).max(0.0))
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

/// Vertical margin a number row's field keeps from the row's top and bottom edges, so the field
/// reads as sitting in the row rather than filling it.
const NUMBER_FIELD_MARGIN: f32 = 3.0;

/// Where a number row's field sits: against the row's right edge, the label taking what is left.
///
/// Shared by layout and hit testing, so the field is laid out exactly where the panel leaves room
/// for it.
pub(crate) fn number_field_bounds(metrics: &PanelMetrics, row_bounds: Rectangle) -> Rectangle {
    let right = row_bounds.x + row_bounds.width - metrics.panel_padding - metrics.row_label_inset;
    let height = (row_bounds.height - NUMBER_FIELD_MARGIN * 2.0).max(0.0);

    Rectangle {
        x: right - metrics.number_input_width,
        y: row_bounds.y + NUMBER_FIELD_MARGIN,
        width: metrics.number_input_width,
        height,
    }
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

/// Draws a [`MenuNode::Toggle`] checkbox centered in the icon slot at `slot_left_x`.
///
/// The box tracks `label_color`, so it follows the row's hover / disabled state instead of
/// fighting the row highlight with a fill of its own.
fn draw_toggle_box<Renderer>(
    renderer: &mut Renderer,
    metrics: &PanelMetrics,
    on: bool,
    label_color: Color,
    row_bounds: Rectangle,
    slot_left_x: f32,
    clip_bounds: Rectangle,
) where
    Renderer: text::Renderer<Font = Font> + svg::Renderer,
{
    let size = metrics.toggle_box_size.min(row_bounds.height);
    let bounds = Rectangle {
        x: slot_left_x + (metrics.icon_slot_width - size) * 0.5,
        y: row_bounds.y + (row_bounds.height - size) * 0.5,
        width: size,
        height: size,
    };

    renderer.fill_quad(
        renderer::Quad {
            bounds,
            border: Border {
                width: 1.0,
                color: Color {
                    a: label_color.a * 0.7,
                    ..label_color
                },
                radius: Radius::from(3.0),
            },
            ..renderer::Quad::default()
        },
        Color::TRANSPARENT,
    );

    if on {
        let inset = size * 0.18;
        renderer.draw_svg(
            svg::Svg::new(check_handle()).color(label_color),
            Rectangle {
                x: bounds.x + inset,
                y: bounds.y + inset,
                width: size - inset * 2.0,
                height: size - inset * 2.0,
            },
            clip_bounds,
        );
    }
}

/// Where a slider row sits and what it is drawn in, gathered from the panel the row belongs to.
struct SliderRow {
    bounds: Rectangle,
    /// Width the icon column takes from the row, zero when the panel has none.
    icon_col: f32,
    /// Whether the row's own icon is drawn in that column.
    icons_enabled: bool,
    /// Whether the pointer is on this slider — over its row, or dragging its handle. The handle
    /// answers with a halo, which is the only mark a slider row takes: it is a dial, not a
    /// command, so it never wears the row highlight the pointer gives every other row, and it
    /// wears nothing at all once the pointer has left.
    active: bool,
    colors: SliderColors,
}

/// Row colors a slider draws itself from, so groove, dots and handle follow the row's label
/// color instead of fighting the row highlight with a palette of their own.
struct SliderColors {
    /// The row's label color: the handle, and the label and value above it.
    label: Color,
    /// The muted color of the scale under the groove.
    edge: Color,
    /// Whatever the row is drawn on, ringed around the handle to lift it off the groove.
    behind_handle: Color,
}

/// Alpha the groove, the filled part of it, the stop dots and the handle's halo are drawn at, as
/// fractions of the row's label color.
const GROOVE_ALPHA: f32 = 0.28;
const GROOVE_FILL_ALPHA: f32 = 0.62;
const DOT_ALPHA: f32 = 0.85;
const HALO_ALPHA: f32 = 0.22;

fn faded(color: Color, alpha: f32) -> Color {
    Color {
        a: color.a * alpha,
        ..color
    }
}

fn rounded(bounds: Rectangle) -> Border {
    Border {
        width: 0.0,
        color: Color::TRANSPARENT,
        radius: Radius::from(bounds.height * 0.5),
    }
}

/// A disc of `size` centered on `center`, which is how the dots, the halo and the handle are all
/// drawn.
fn disc(center: Point, size: f32) -> Rectangle {
    Rectangle {
        x: center.x - size * 0.5,
        y: center.y - size * 0.5,
        width: size,
        height: size,
    }
}

/// One value of a slider's scale as it is drawn: the stop it belongs to and the strip of the row
/// it is laid in.
struct ScaleLabel {
    index: usize,
    left: f32,
    width: f32,
}

impl ScaleLabel {
    fn right(&self) -> f32 {
        self.left + self.width
    }
}

/// The stops whose value is spelled out under the dots: both ends always, and as many of the
/// stops between them as fit without crowding.
///
/// A label is centered under its own dot, except at the ends, where it is pulled back inside the
/// groove so the scale stays within the row's text column.
fn scale_labels<Renderer: text::Renderer>(
    renderer: &Renderer,
    metrics: &PanelMetrics,
    track: &SliderTrack,
    stops: &[SliderStop<'_>],
) -> Vec<ScaleLabel> {
    let count = stops.len();
    if count == 0 {
        return Vec::new();
    }
    let (lane_left, lane_right) = track.lane;
    let placed = |index: usize| {
        let width = measure_hotkey_width(renderer, metrics, stops[index].label.as_ref());
        let center = track.stop_x(count, index);
        ScaleLabel {
            index,
            left: (center - width * 0.5).clamp(lane_left, (lane_right - width).max(lane_left)),
            width,
        }
    };

    let mut labels = vec![placed(0)];
    if count == 1 {
        return labels;
    }

    let last = placed(count - 1);
    for index in 1..count - 1 {
        let label = placed(index);
        let clears_previous = labels
            .last()
            .is_some_and(|previous| label.left >= previous.right() + metrics.slider_label_gap);
        if clears_previous && label.right() + metrics.slider_label_gap <= last.left {
            labels.push(label);
        }
    }
    labels.push(last);
    labels
}

/// Draws a [`MenuNode::Slider`] row: its label on the upper line, then the groove with a dot at
/// each stop, and the values those stops stand for.
fn draw_slider<Renderer>(
    renderer: &mut Renderer,
    metrics: &PanelMetrics,
    row: &SliderRow,
    node: &MenuNode<'_>,
    clip_bounds: Rectangle,
) where
    Renderer: text::Renderer<Font = Font> + svg::Renderer,
{
    let MenuNode::Slider {
        title,
        stops,
        selected,
        icon,
        ..
    } = node
    else {
        return;
    };

    let SliderRow {
        bounds,
        icon_col,
        icons_enabled,
        active,
        colors,
    } = row;
    let font = renderer.default_font();
    let content_left = bounds.x + metrics.panel_padding + metrics.row_label_inset;
    let lane_left = content_left + icon_col;

    let title_band = Rectangle {
        height: metrics.row_height.min(bounds.height),
        ..*bounds
    };
    // Every line of a slider is set from the left, the label against the text column and each
    // scale value against the strip [`scale_labels`] laid it in.
    let line = |size: f32, content: String, band: Rectangle| text::Text {
        content,
        bounds: Size::new(band.width, band.height),
        size: Pixels(size),
        line_height: text::LineHeight::default(),
        font,
        align_x: text::Alignment::Left,
        align_y: alignment::Vertical::Center,
        shaping: text::Shaping::default(),
        wrapping: text::Wrapping::None,
    };

    if let Some(ic) = icon.as_ref().filter(|_| *icons_enabled) {
        draw_row_icon(
            renderer,
            metrics,
            ic,
            title_band,
            content_left,
            clip_bounds,
            colors.label,
        );
    }
    renderer.fill_text(
        line(metrics.label_size, title.as_ref().to_string(), title_band),
        Point::new(lane_left, title_band.center_y()),
        colors.label,
        clip_bounds,
    );
    let track = slider_track(metrics, *icon_col, *bounds);
    renderer.fill_quad(
        renderer::Quad {
            bounds: track.groove,
            border: rounded(track.groove),
            ..renderer::Quad::default()
        },
        faded(colors.label, GROOVE_ALPHA),
    );

    let handle_x = track.stop_x(stops.len(), *selected);
    let filled = Rectangle {
        width: (handle_x - track.groove.x).max(0.0),
        ..track.groove
    };
    renderer.fill_quad(
        renderer::Quad {
            bounds: filled,
            border: rounded(filled),
            ..renderer::Quad::default()
        },
        faded(colors.label, GROOVE_FILL_ALPHA),
    );

    if stops.len() > 1 {
        for index in 0..stops.len() {
            let dot = disc(
                Point::new(track.stop_x(stops.len(), index), track.groove.center_y()),
                metrics.slider_dot_size,
            );
            renderer.fill_quad(
                renderer::Quad {
                    bounds: dot,
                    border: rounded(dot),
                    ..renderer::Quad::default()
                },
                faded(colors.label, DOT_ALPHA),
            );
        }
    }

    for label in scale_labels(renderer, metrics, &track, stops) {
        let color = if label.index == *selected {
            colors.label
        } else {
            colors.edge
        };
        renderer.fill_text(
            line(
                metrics.hotkey_label_size,
                stops[label.index].label.as_ref().to_string(),
                Rectangle {
                    width: label.width,
                    ..track.band
                },
            ),
            Point::new(label.left, track.scale_y),
            color,
            clip_bounds,
        );
    }

    // The handle grows a soft edge under the pointer and is a plain dot the rest of the time.
    let center = Point::new(handle_x, track.groove.center_y());
    if *active && metrics.slider_handle_halo > 0.0 {
        let halo = disc(
            center,
            metrics.slider_handle_size + metrics.slider_handle_halo * 2.0,
        );
        renderer.fill_quad(
            renderer::Quad {
                bounds: halo,
                border: rounded(halo),
                ..renderer::Quad::default()
            },
            faded(colors.label, HALO_ALPHA),
        );
    }
    let handle = disc(center, metrics.slider_handle_size);
    renderer.fill_quad(
        renderer::Quad {
            bounds: handle,
            border: Border {
                width: 1.0,
                color: colors.behind_handle,
                radius: Radius::from(metrics.slider_handle_size * 0.5),
            },
            ..renderer::Quad::default()
        },
        colors.label,
    );
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
    // Path of the slider row a drag is holding, empty when none is.
    drag_path: &[usize],
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
    let icons_enabled = icons_column_enabled(nodes, icons_enabled);
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
        // A slider row is left unhighlighted: the handle's halo says the pointer is on it, and a
        // wash behind a groove drawn from the same label color only muddies both.
        let show_row_highlight =
            !matches!(
                node,
                MenuNode::Separator | MenuNode::Slider { .. } | MenuNode::Number { .. }
            ) && (hovered || open_chain || (pointer_row.is_none() && is_focused));

        let pressed = false;
        let row_label_color = |enabled: bool| {
            if show_row_highlight {
                style.row_hover_label_color
            } else if !enabled {
                style.disabled_color
            } else {
                style.label_color
            }
        };
        let row_hotkey_color = |enabled: bool| {
            if show_row_highlight {
                style.row_hover_hotkey_label_color
            } else if !enabled {
                style.disabled_color
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
                hotkey,
                ..
            }
            | MenuNode::Toggle {
                title,
                enabled,
                hotkey,
                ..
            } => {
                let color = row_label_color(*enabled);
                // A toggle spends the icon slot on its checkbox; an action draws its icon there.
                match node {
                    MenuNode::Toggle { on, .. } => draw_toggle_box(
                        renderer,
                        metrics,
                        *on,
                        color,
                        row_bounds,
                        row_content_left(row_bounds),
                        clip_bounds,
                    ),
                    MenuNode::Action { icon: Some(ic), .. } if icons_enabled => draw_row_icon(
                        renderer,
                        metrics,
                        ic,
                        row_bounds,
                        row_content_left(row_bounds),
                        clip_bounds,
                        color,
                    ),
                    _ => {}
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
            MenuNode::Slider { enabled, .. } => {
                draw_slider(
                    renderer,
                    metrics,
                    &SliderRow {
                        bounds: row_bounds,
                        icon_col,
                        icons_enabled,
                        active: *enabled && (hovered || drag_path == row_path.as_slice()),
                        colors: SliderColors {
                            label: row_label_color(*enabled),
                            edge: row_hotkey_color(*enabled),
                            behind_handle: if show_row_highlight {
                                style.row_hover_background
                            } else {
                                style.panel_background
                            },
                        },
                    },
                    node,
                    clip_bounds,
                );
            }
            MenuNode::Number {
                title,
                enabled,
                icon,
                ..
            } => {
                let color = row_label_color(*enabled);
                if icons_enabled && let Some(ic) = icon {
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
                let label_x = label_x_for_row(row_bounds);
                let field = number_field_bounds(metrics, row_bounds);
                renderer.fill_text(
                    text::Text {
                        content: title.as_ref().to_string(),
                        bounds: Size::new(
                            (field.x - metrics.label_hotkey_gap - label_x).max(0.0),
                            row_bounds.height,
                        ),
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
                renderer.draw_svg(svg::Svg::new(handle).color(color), svg_bounds, clip_bounds);
            }
        }
    }

    let _ = depth;
}
