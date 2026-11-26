use incular_config::{Axis, Constraints};
use incular_core::{Color, Size};
use incular_layout::AspectRatio as Ratio;
use incular_widgets::{
    AspectRatio, IntrinsicHeight, IntrinsicWidth, OverflowBox, SizedBox, Widget,
    internal::{Key, TreeError, WidgetTree},
};

fn intrinsic_ratio(aspect: f32, child: Option<Size>) {
    let ratio = Ratio::new(aspect);
    for (opposite, expected_width, expected_height) in if aspect == 2. {
        [(200., 400., 100.), (400., 800., 200.)]
    } else {
        [(200., 100., 400.), (400., 200., 800.)]
    } {
        // The min/max finite branches use the same aspect equation.
        assert_eq!(
            ratio.intrinsic_extent(Axis::Horizontal, opposite),
            Some(expected_width)
        );
        assert_eq!(
            ratio.intrinsic_extent(Axis::Vertical, opposite),
            Some(expected_height)
        );
    }
    let wanted = child.unwrap_or(Size::ZERO);
    for axis in [Axis::Horizontal, Axis::Vertical] {
        assert_eq!(ratio.intrinsic_extent(axis, f32::INFINITY), None);
    }

    // The actual render proxy uses the finite ratio or the child's width
    // through intrinsic sizing, not through a speculative unbounded layout.
    let child = child.map_or_else(
        || SizedBox::shrink().into(),
        |size| Widget::box_(size, Color::WHITE),
    );
    for max_height in [200., f32::INFINITY] {
        let mut tree = WidgetTree::new();
        tree.mount(IntrinsicWidth::new(AspectRatio::new(aspect, child.clone())).into())
            .unwrap();
        tree.layout(Constraints::at_most(1200., max_height))
            .unwrap();
        let expected = if max_height.is_finite() {
            max_height * aspect
        } else {
            wanted.width
        };
        assert_eq!(tree.root_layout_size().unwrap().width, expected);
    }
    for max_width in [200., f32::INFINITY] {
        let mut tree = WidgetTree::new();
        tree.mount(IntrinsicHeight::new(AspectRatio::new(aspect, child.clone())).into())
            .unwrap();
        tree.layout(Constraints::at_most(max_width, 1200.)).unwrap();
        let expected = if max_width.is_finite() {
            max_width / aspect
        } else {
            wanted.height
        };
        assert_eq!(tree.root_layout_size().unwrap().height, expected);
    }
}

#[test]
fn landscape_intrinsics_without_a_child_use_zero_when_unbounded() {
    intrinsic_ratio(2., None);
}

#[test]
fn portrait_intrinsics_without_a_child_use_zero_when_unbounded() {
    intrinsic_ratio(0.5, None);
}

#[test]
fn landscape_intrinsics_fall_back_to_the_child_only_for_an_infinite_opposite_axis() {
    intrinsic_ratio(2., Some(Size::new(90., 70.)));
}

#[test]
fn portrait_intrinsics_fall_back_to_the_child_only_for_an_infinite_opposite_axis() {
    intrinsic_ratio(0.5, Some(Size::new(90., 70.)));
}

#[test]
fn live_aspect_ratio_obeys_tight_bounded_and_one_sided_resize_constraints() {
    const KEY: Key = Key::Value(350);
    let widget =
        |ratio: f32| Widget::from(AspectRatio::new(ratio, SizedBox::shrink())).with_key(KEY);
    let mut tree = WidgetTree::new();
    let root = tree.mount(OverflowBox::new(widget(1.)).into()).unwrap();
    let host = Constraints::tight(Size::new(800., 600.));
    tree.layout(host).unwrap();
    let element = tree.element_with_key(&KEY).unwrap();
    assert_eq!(
        tree.element_bounds(element).unwrap().size,
        Size::new(800., 600.)
    );

    for (ratio, minimum, maximum, expected) in [
        (1., (0., 0.), (100., 90.), (90., 90.)),
        (1., (0., 0.), (90., 100.), (90., 90.)),
        (1., (0., 0.), (f32::INFINITY, 90.), (90., 90.)),
        (1., (0., 0.), (90., f32::INFINITY), (90., 90.)),
        (2., (0., 0.), (100., 90.), (100., 50.)),
        (2., (0., 0.), (90., 100.), (90., 45.)),
        (2., (0., 0.), (f32::INFINITY, 90.), (180., 90.)),
        (2., (0., 0.), (90., f32::INFINITY), (90., 45.)),
        (2., (80., 80.), (100., 90.), (100., 80.)),
        (2., (80., 80.), (90., 100.), (90., 80.)),
        (2., (80., 80.), (f32::INFINITY, 90.), (180., 90.)),
        (2., (80., 80.), (90., f32::INFINITY), (90., 80.)),
        (2., (0., 40.), (f32::INFINITY, 40.), (80., 40.)),
    ] {
        tree.update(
            root,
            OverflowBox::new(widget(ratio))
                .min_width(minimum.0)
                .min_height(minimum.1)
                .max_width(maximum.0)
                .max_height(maximum.1)
                .into(),
        )
        .unwrap();
        tree.layout(host).unwrap();
        assert_eq!(tree.element_with_key(&KEY), Some(element));
        assert_eq!(
            tree.element_bounds(element).unwrap().size,
            Size::new(expected.0, expected.1),
            "ratio={ratio}, minima={minimum:?}, maxima={maximum:?}"
        );
    }
}

#[test]
fn both_unbounded_axes_report_an_error_instead_of_laying_out_at_zero() {
    let mut tree = WidgetTree::new();
    tree.mount(
        OverflowBox::new(AspectRatio::new(
            0.5,
            Widget::box_(Size::new(90., 70.), Color::WHITE),
        ))
        .min_width(0.)
        .max_width(f32::INFINITY)
        .min_height(0.)
        .max_height(f32::INFINITY)
        .into(),
    )
    .unwrap();
    let error = tree
        .layout(Constraints::tight(Size::new(800., 600.)))
        .unwrap_err();
    assert!(matches!(
        &error,
        TreeError::InvalidWidgetConfiguration {
            widget: "AspectRatio",
            ..
        }
    ));
    assert!(
        error
            .to_string()
            .contains("aspect ratio 0.5 has unbounded width and height")
    );
}
