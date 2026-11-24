//! Flutter 3.50.0 retained Wrap geometry and hit behavior, paired with
//! the named registrations in packages/flutter/test/widgets/wrap_test.dart.

use std::cell::Cell;
use std::rc::Rc;
use std::time::{Duration, Instant};

use incular_config::{Alignment, Axis, Constraints, WrapAlignment};
use incular_core::{
    InputEvent, Offset, PRIMARY_POINTER_BUTTON, PointerDeviceKind, PointerPhase,
    PointerSampleMetadata, Rect, Size,
};
use incular_runtime::Runtime;
use incular_widgets::{
    Align, Center, Directionality, GestureDetector, HitTestBehavior, Placeholder, SizedBox,
    TextDirection, Widget, Wrap,
    internal::{Key, WidgetTree},
};

const WRAP: u64 = 100;

fn viewport() -> Constraints {
    Constraints::tight(Size::new(800., 600.))
}

fn bounds(tree: &WidgetTree, key: u64) -> Rect {
    let element = tree.element_with_key(&Key::from(key)).expect("fixture key");
    tree.element_bounds(element).expect("fixture bounds")
}

fn keyed_boxes(sizes: impl IntoIterator<Item = (f32, f32)>) -> impl Iterator<Item = Widget> {
    sizes.into_iter().enumerate().map(|(i, (width, height))| {
        Widget::from(SizedBox::new().width(width).height(height)).with_key(i as u64)
    })
}

fn top_left(wrap: Wrap) -> Widget {
    Align::new(Alignment::TOP_LEFT, Widget::from(wrap).with_key(WRAP)).into()
}

fn check_geometry(tree: &WidgetTree, size: Size, origins: &[(f32, f32)]) {
    assert_eq!(bounds(tree, WRAP).size, size);
    for (i, &(x, y)) in origins.iter().enumerate() {
        assert_eq!(
            bounds(tree, i as u64).origin,
            Offset::new(x, y),
            "child {i}"
        );
    }
}

// wrap_test.dart:694, "Wrap spacing test".
#[test]
fn horizontal_wrap_places_a_gap_between_each_unequal_height_run() {
    let wrap = Wrap::new(keyed_boxes(
        [10., 20., 30., 40.].map(|height| (500., height)),
    ))
    .run_spacing(10.)
    .text_direction(TextDirection::Ltr);
    let mut tree = WidgetTree::new();
    tree.mount(top_left(wrap)).expect("mount");
    tree.layout(viewport()).expect("layout");
    check_geometry(
        &tree,
        Size::new(500., 130.),
        &[(0., 0.), (0., 20.), (0., 50.), (0., 90.)],
    );
}

// wrap_test.dart:722, "Vertical Wrap test with spacing", includes the
// upstream repump as horizontal with different spacing in the same tree.
#[test]
fn vertical_wrap_can_switch_to_horizontal_with_the_expected_spacing() {
    let build = |axis, spacing, run_spacing| {
        top_left(
            Wrap::new(keyed_boxes(
                [10., 20., 30., 40., 50., 60.].map(|width| (width, 250.)),
            ))
            .direction(axis)
            .spacing(spacing)
            .run_spacing(run_spacing)
            .text_direction(TextDirection::Ltr),
        )
    };
    let mut tree = WidgetTree::new();
    let root = tree.mount(build(Axis::Vertical, 10., 15.)).expect("mount");
    tree.layout(viewport()).expect("vertical layout");
    check_geometry(
        &tree,
        Size::new(150., 510.),
        &[
            (0., 0.),
            (0., 260.),
            (35., 0.),
            (35., 260.),
            (90., 0.),
            (90., 260.),
        ],
    );
    tree.update(root, build(Axis::Horizontal, 12., 8.))
        .expect("switch direction");
    tree.layout(viewport()).expect("horizontal layout");
    check_geometry(
        &tree,
        Size::new(270., 250.),
        &[
            (0., 0.),
            (22., 0.),
            (54., 0.),
            (96., 0.),
            (148., 0.),
            (210., 0.),
        ],
    );
}

// wrap_test.dart:811, "Hit test children in wrap".
#[test]
fn wrap_taps_respect_the_item_and_run_spacing_at_the_child_corner() {
    let taps = Rc::new(Cell::new(0));
    let mut children: Vec<Widget> = (0..4)
        .map(|_| SizedBox::new().width(200.).height(300.).into())
        .collect();
    let target = GestureDetector::new(SizedBox::shrink())
        .behavior(HitTestBehavior::Opaque)
        .on_tap({
            let taps = Rc::clone(&taps);
            move || taps.set(taps.get() + 1)
        });
    children.push(
        SizedBox::new()
            .width(200.)
            .height(300.)
            .child(target)
            .into(),
    );
    let mut runtime = Runtime::new(
        Wrap::new(children)
            .spacing(10.)
            .run_spacing(15.)
            .text_direction(TextDirection::Ltr)
            .into(),
    )
    .expect("mount");
    let start = Instant::now();
    runtime
        .run_frame_at(viewport(), start)
        .expect("first frame");
    for (i, (x, y)) in [(209., 314.), (211., 314.), (209., 316.), (211., 316.)]
        .into_iter()
        .enumerate()
    {
        let now = start + Duration::from_millis(30 * (i as u64 + 1));
        for (phase, buttons, millis) in [
            (PointerPhase::Down, PRIMARY_POINTER_BUTTON, 0),
            (PointerPhase::Up, 0, 1),
        ] {
            let _ = runtime.handle_input_at(
                InputEvent::PointerWithMetadata {
                    pointer: i as u64 + 1,
                    device: 0,
                    kind: PointerDeviceKind::Touch,
                    buttons,
                    button: Some(PRIMARY_POINTER_BUTTON),
                    sample: PointerSampleMetadata::default(),
                    phase,
                    position: Offset::new(x, y),
                },
                now + Duration::from_millis(millis),
            );
        }
        runtime
            .run_frame_at(viewport(), now + Duration::from_millis(2))
            .expect("input frame");
        assert_eq!(taps.get(), u32::from(i == 3), "tap at ({x}, {y})");
    }
}

// wrap_test.dart:1030, "Wrap alignment flipped spaceInBetween".
#[test]
fn rtl_space_between_aligns_a_single_item_to_the_right() {
    let wrap = Wrap::new(keyed_boxes([(100., 100.)]))
        .text_direction(TextDirection::Rtl)
        .alignment(WrapAlignment::SpaceBetween);
    let mut tree = WidgetTree::new();
    tree.mount(Widget::from(wrap).with_key(WRAP))
        .expect("mount");
    tree.layout(viewport()).expect("layout");
    check_geometry(&tree, Size::new(800., 600.), &[(700., 0.)]);
}

// wrap_test.dart:1045, "Wrap does not crash at zero area".
#[test]
fn wrap_placeholder_at_zero_viewport_size_can_complete_a_frame() {
    let wrap = Widget::from(Wrap::new([Placeholder::new()])).with_key(WRAP);
    let mut runtime =
        Runtime::new(Directionality::new(TextDirection::Ltr, Center::new(wrap)).into())
            .expect("mount");
    runtime
        .run_frame(Constraints::tight(Size::ZERO))
        .expect("frame");
    assert_eq!(bounds(runtime.tree(), WRAP).size, Size::ZERO);
}
