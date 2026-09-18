//! The [`MenuNode::Number`] rows of one panel: the real [`NumberInput`] each of them carries, and
//! the widget state those inputs keep between frames.
//!
//! Every other row of the menu is painted by [`super::panel`] from plain data. A number row is the
//! one exception: it holds a text field, and a field is selection, caret, clipboard and a draft
//! that has to survive the frame it is typed in. Reimplementing that inside a painted row would be
//! reimplementing `text_input`, so the row hosts the real widget instead and the overlay drives it
//! like any parent would.
//!
//! The spec is rebuilt every frame, so the elements are too; what cannot be rebuilt is their
//! [`Tree`], which is where the draft lives. Those are keyed by panel and row in [`Fields`], which
//! the widget owns and hands down to each overlay.

use super::menu::{MenuItemId, MenuNode};
use super::panel::PanelMetrics;

use std::collections::HashMap;

use iced::advanced::text::Renderer as TextRenderer;
use iced::advanced::widget::Tree;
use iced::{Element, Font};
use iced_numbers_input::{Catalog, Icon, NumberInput, Orientation};

/// A number row's field, addressed by the panel it is on and its index in that panel.
///
/// The panel is `0` for the root and `depth + 1` for a flyout, so a row of one panel can never be
/// read as a row of another.
pub(crate) type FieldKey = (usize, usize);

/// The value a field reports, before the menu turns it into the application's message. Owned by
/// the field so its `on_change` can stay `'static`; the borrowed callback is applied by the `map`
/// that follows.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Reported(f64);

/// Widget state of every number field the menu has shown while open.
///
/// Rows come and go as the spec is rebuilt, so an entry outlives the row that made it. They are
/// dropped together when the menu closes, which is also when a draft stops meaning anything.
#[derive(Debug, Default)]
pub(crate) struct Fields {
    trees: HashMap<FieldKey, Tree>,
}

impl Fields {
    pub(crate) fn clear(&mut self) {
        self.trees.clear();
    }

    /// Reconciles the tree for `key` against `element` — created on the first frame the row is
    /// shown, carried across every frame after it.
    pub(crate) fn reconcile<Message, Theme, Renderer>(
        &mut self,
        key: FieldKey,
        element: &Element<'_, Message, Theme, Renderer>,
    ) where
        Renderer: iced::advanced::Renderer,
    {
        let tree = self
            .trees
            .entry(key)
            .or_insert_with(|| Tree::new(element.as_widget()));
        tree.diff(element.as_widget());
    }

    pub(crate) fn tree(&self, key: FieldKey) -> Option<&Tree> {
        self.trees.get(&key)
    }

    pub(crate) fn tree_mut(&mut self, key: FieldKey) -> Option<&mut Tree> {
        self.trees.get_mut(&key)
    }
}

/// How a panel's number fields are built.
#[derive(Debug, Clone)]
pub(crate) struct FieldStyle {
    pub orientation: Orientation,
    pub icons: Option<(Icon, Icon)>,
}

/// The number rows of `nodes`, each with its index in the panel.
pub(crate) fn rows<'n, 'a>(
    nodes: &'n [MenuNode<'a>],
) -> impl Iterator<Item = (usize, &'n MenuNode<'a>)> {
    nodes
        .iter()
        .enumerate()
        .filter(|(_, node)| matches!(node, MenuNode::Number { .. }))
}

/// The field a number row carries, reporting through `on_number`.
///
/// A whole-number row is built over `i64` and a fractional one over `f64`, so the field itself
/// refuses a decimal point on a row that counts pixels rather than accepting one and rounding it
/// away behind the user's back.
pub(crate) fn field<'a, Message, Theme, Renderer>(
    node: &MenuNode<'_>,
    metrics: &PanelMetrics,
    style: &FieldStyle,
    on_number: &'a (dyn Fn(MenuItemId, f64) -> Message + 'a),
) -> Option<Element<'a, Message, Theme, Renderer>>
where
    Message: 'a + Clone,
    Theme: 'a + Catalog,
    Renderer: 'a + TextRenderer<Font = Font>,
{
    let MenuNode::Number {
        id,
        value,
        min,
        max,
        step,
        integral,
        enabled,
        ..
    } = node
    else {
        return None;
    };

    // A field whose bounds are a single point ignores every edit, which is what a disabled row
    // has to do.
    let disabled = !enabled;
    let element: Element<'a, Reported, Theme, Renderer> = if *integral {
        let value = value.round() as i64;
        let (min, max) = if disabled {
            (value, value)
        } else {
            (min.round() as i64, max.round() as i64)
        };
        configure(
            NumberInput::new(value, min..=max, |value: i64| Reported(value as f64)),
            metrics,
            style,
        )
        .step(step.round().max(1.0) as i64)
        .into()
    } else {
        let (min, max) = if disabled {
            (*value, *value)
        } else {
            (*min, *max)
        };
        configure(
            NumberInput::new(*value, min..=max, Reported),
            metrics,
            style,
        )
        .step(*step)
        .into()
    };

    let id = *id;
    Some(element.map(move |Reported(value)| on_number(id, value)))
}

/// The knobs every field takes from the menu, whatever number type it holds.
fn configure<'a, T, Theme, Renderer>(
    input: NumberInput<'a, T, Reported, Theme, Renderer>,
    metrics: &PanelMetrics,
    style: &FieldStyle,
) -> NumberInput<'a, T, Reported, Theme, Renderer>
where
    T: num_traits::Num
        + num_traits::NumAssignOps
        + PartialOrd
        + std::fmt::Display
        + std::str::FromStr
        + Copy
        + num_traits::Bounded,
    Theme: Catalog,
    Renderer: TextRenderer<Font = Font>,
{
    let input = input
        .width(metrics.number_input_width)
        .text_size(metrics.number_text_size)
        .stepper_width(metrics.number_stepper_width)
        .padding(metrics.number_padding)
        .orientation(style.orientation);

    match &style.icons {
        Some((increase, decrease)) => input.icons(increase.clone(), decrease.clone()),
        None => input,
    }
}
