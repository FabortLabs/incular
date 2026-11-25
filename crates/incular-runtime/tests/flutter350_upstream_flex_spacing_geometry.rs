//! Pinned Flutter RenderFlex placement, tested through retained widgets.
use incular_config::{
    Axis, Constraints, CrossAxisAlignment, MainAxisAlignment, MainAxisSize, TextDirection,
    VerticalDirection,
};
use incular_core::{Rect, Size};
use incular_runtime::Runtime;
use incular_widgets::{Expanded, Flex, SizedBox, Widget, internal::Key};

fn child(index: u64, size: Size) -> Widget {
    Widget::from(SizedBox::from_size(size)).with_key(Key::Value(index))
}

fn positions(runtime: &Runtime) -> [Rect; 3] {
    [0, 1, 2].map(|i| {
        let tree = runtime.tree();
        tree.element_bounds(tree.element_with_key(&Key::Value(i)).unwrap())
            .unwrap()
    })
}

fn trace(alignment: MainAxisAlignment, spacing: f32, horizontal: [f32; 3], vertical: [f32; 3]) {
    let build = |axis| -> Widget {
        Flex::new(axis, [0, 1, 2].map(|i| child(i, Size::new(100., 100.))))
            .text_direction(TextDirection::Ltr)
            .main_axis_alignment(alignment)
            .spacing(spacing)
            .into()
    };
    let viewport = Constraints::at_most(500., 400.);
    let mut runtime = Runtime::new(build(Axis::Horizontal)).unwrap();
    runtime.run_frame(viewport).unwrap();
    let before = positions(&runtime);
    let identities = [0, 1, 2].map(|i| runtime.tree().element_with_key(&Key::Value(i)));
    let root = runtime.tree().root().unwrap();
    runtime
        .schedule_update(root, build(Axis::Vertical))
        .unwrap();
    runtime.run_frame(viewport).unwrap();
    let after = positions(&runtime);
    assert_eq!(
        identities,
        [0, 1, 2].map(|i| runtime.tree().element_with_key(&Key::Value(i)))
    );
    for i in 0..3 {
        let x = before[i].origin.x;
        let y = after[i].origin.y;
        assert!((x - horizontal[i]).abs() < 0.001, "row child {i}: {x}");
        assert!((y - vertical[i]).abs() < 0.001, "column child {i}: {y}");
        assert_eq!(before[i].origin.y, 0.);
        assert_eq!(after[i].origin.x, 0.);
        assert_eq!(before[i].size, Size::new(100., 100.));
        assert_eq!(after[i].size, Size::new(100., 100.));
    }
}

#[test]
fn space_evenly_distributes_both_outer_gaps() {
    trace(
        MainAxisAlignment::SpaceEvenly,
        0.,
        [50., 200., 350.],
        [25., 150., 275.],
    );
}

#[test]
fn start_alignment_preserves_interchild_spacing() {
    trace(
        MainAxisAlignment::Start,
        14.,
        [0., 114., 228.],
        [0., 114., 228.],
    );
}

#[test]
fn end_alignment_preserves_interchild_spacing() {
    trace(
        MainAxisAlignment::End,
        14.,
        [172., 286., 400.],
        [72., 186., 300.],
    );
}

#[test]
fn center_alignment_preserves_interchild_spacing() {
    trace(
        MainAxisAlignment::Center,
        14.,
        [86., 200., 314.],
        [36., 150., 264.],
    );
}

#[test]
fn space_evenly_adds_fixed_spacing_only_between_children() {
    trace(
        MainAxisAlignment::SpaceEvenly,
        14.,
        [43., 200., 357.],
        [18., 150., 282.],
    );
}

#[test]
fn space_around_adds_fixed_spacing_only_between_children() {
    trace(
        MainAxisAlignment::SpaceAround,
        14.,
        [28.666_666, 200., 371.333_34],
        [12., 150., 288.],
    );
}

#[test]
fn changing_flex_parent_data_recomputes_the_child_share() {
    let build = |factor| -> Widget {
        Flex::horizontal([
            child(0, Size::ZERO),
            Expanded::flex_factor(factor, child(1, Size::ZERO)).into(),
        ])
        .text_direction(TextDirection::Ltr)
        .into()
    };
    let mut runtime = Runtime::new(build(0)).unwrap();
    let viewport = Constraints::at_most(100., 100.);
    runtime.run_frame(viewport).unwrap();
    let tree = runtime.tree();
    let first = tree.element_with_key(&Key::Value(0)).unwrap();
    let second = tree.element_with_key(&Key::Value(1)).unwrap();
    assert_eq!(tree.element_bounds(first).unwrap().size, Size::ZERO);
    assert_eq!(tree.element_bounds(second).unwrap().size, Size::ZERO);

    let root = tree.root().unwrap();
    runtime.schedule_update(root, build(1)).unwrap();
    runtime.run_frame(viewport).unwrap();
    let tree = runtime.tree();
    assert_eq!(tree.element_with_key(&Key::Value(1)), Some(second));
    assert_eq!(tree.element_bounds(first).unwrap().size, Size::ZERO);
    assert_eq!(
        tree.element_bounds(second).unwrap().size,
        Size::new(100., 0.)
    );
}

#[test]
fn stretch_tightens_cross_axis_without_altering_main_axis_shares() {
    let build = |axis, cross| -> Widget {
        Flex::new(
            axis,
            [
                child(0, Size::ZERO),
                Expanded::flex_factor(2, child(1, Size::ZERO)).into(),
            ],
        )
        .text_direction(TextDirection::Ltr)
        .cross_axis_alignment(cross)
        .into()
    };
    let viewport = Constraints::at_most(100., 100.);
    let mut runtime = Runtime::new(build(Axis::Horizontal, CrossAxisAlignment::Center)).unwrap();
    let measure = |runtime: &Runtime| {
        [0, 1].map(|i| {
            let tree = runtime.tree();
            tree.element_bounds(tree.element_with_key(&Key::Value(i)).unwrap())
                .unwrap()
                .size
        })
    };
    runtime.run_frame(viewport).unwrap();
    assert_eq!(measure(&runtime), [Size::ZERO, Size::new(100., 0.)]);
    let root = runtime.tree().root().unwrap();
    runtime
        .schedule_update(root, build(Axis::Horizontal, CrossAxisAlignment::Stretch))
        .unwrap();
    runtime.run_frame(viewport).unwrap();
    assert_eq!(
        measure(&runtime),
        [Size::new(0., 100.), Size::new(100., 100.)]
    );

    runtime
        .schedule_update(root, build(Axis::Vertical, CrossAxisAlignment::Stretch))
        .unwrap();
    runtime.run_frame(viewport).unwrap();
    assert_eq!(
        measure(&runtime),
        [Size::new(100., 0.), Size::new(100., 100.)]
    );
}

#[test]
fn retained_flex_defaults_match_the_public_layout_values() {
    let flex = Flex::default();
    assert_eq!(flex.direction, Axis::Horizontal);
    assert_eq!(flex.main_axis_alignment, MainAxisAlignment::Start);
    assert_eq!(flex.main_axis_size, MainAxisSize::Max);
    assert_eq!(flex.cross_axis_alignment, CrossAxisAlignment::Center);
    assert_eq!(flex.vertical_direction, VerticalDirection::Down);
    assert_eq!(flex.spacing, 0.);
}
