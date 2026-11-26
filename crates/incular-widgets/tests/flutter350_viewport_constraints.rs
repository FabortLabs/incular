//! Viewport constraint diagnostics must run before attaching controllers.

use incular_config::{Axis, Constraints};
use incular_core::Size;
use incular_widgets::{
    Directionality, ListView, ScrollController, SizedBox, TextDirection,
    internal::{TreeError, WidgetTree},
};

fn nested_lists(
    outer_axis: Axis,
    inner_axis: Axis,
    shrink_wrap: bool,
    has_child: bool,
) -> (WidgetTree, ScrollController) {
    let inner_controller = ScrollController::new();
    let inner = ListView::new(has_child.then(|| SizedBox::new().width(50.).height(50.)))
        .controller(inner_controller.clone())
        .scroll_direction(inner_axis)
        .shrink_wrap(shrink_wrap);
    let outer = ListView::new([inner]).scroll_direction(outer_axis);
    let mut tree = WidgetTree::new();
    tree.mount(Directionality::new(TextDirection::Ltr, outer).into())
        .unwrap();
    (tree, inner_controller)
}

fn bounds() -> Constraints {
    Constraints::tight(Size::new(800., 600.))
}

#[cfg(debug_assertions)]
#[test]
fn expanding_viewport_identifies_which_axis_is_unbounded() {
    for (outer, inner, headline, main_axis) in [
        (
            Axis::Vertical,
            Axis::Horizontal,
            "Horizontal viewport was given unbounded height.",
            false,
        ),
        (
            Axis::Horizontal,
            Axis::Horizontal,
            "Horizontal viewport was given unbounded width.",
            true,
        ),
        (
            Axis::Horizontal,
            Axis::Vertical,
            "Vertical viewport was given unbounded width.",
            false,
        ),
        (
            Axis::Vertical,
            Axis::Vertical,
            "Vertical viewport was given unbounded height.",
            true,
        ),
    ] {
        let (mut tree, controller) = nested_lists(outer, inner, false, false);
        let TreeError::InvalidWidgetConfiguration { widget, reason } =
            tree.layout(bounds()).unwrap_err()
        else {
            panic!("expected a viewport constraint diagnostic");
        };
        assert_eq!(widget, "RenderViewport");
        assert!(reason.starts_with(headline), "{reason}");
        let explanation = if main_axis {
            "Viewports expand in the scrolling direction"
        } else {
            "Viewports expand in the cross axis"
        };
        assert!(reason.contains(explanation), "{reason}");
        assert!(!controller.has_clients());
    }
}

#[cfg(debug_assertions)]
#[test]
fn shrink_wrapping_still_requires_a_bounded_cross_axis_even_when_empty() {
    for (outer, inner, has_child) in [
        (Axis::Horizontal, Axis::Vertical, true),
        (Axis::Vertical, Axis::Horizontal, true),
        (Axis::Horizontal, Axis::Vertical, false),
    ] {
        let (mut tree, controller) = nested_lists(outer, inner, true, has_child);
        let TreeError::InvalidWidgetConfiguration { widget, reason } =
            tree.layout(bounds()).unwrap_err()
        else {
            panic!("expected a shrink-wrapping viewport constraint diagnostic");
        };
        assert_eq!(widget, "RenderShrinkWrappingViewport");
        assert!(
            reason.contains("Viewports expand in the cross axis"),
            "{reason}"
        );
        assert!(!controller.has_clients());
    }
}

#[test]
fn shrink_wrapping_accepts_an_unbounded_main_axis_when_cross_axis_is_bounded() {
    for axis in [Axis::Horizontal, Axis::Vertical] {
        let (mut tree, controller) = nested_lists(axis, axis, true, true);
        tree.layout(bounds()).unwrap();
        assert!(controller.has_clients());
        assert_eq!(controller.viewport_extent(), 50.);
        assert_eq!(controller.max_offset(), 0.);
    }
}
