//! Flutter 3.50.0 retained transform pivot and quarter-turn geometry/hit counterparts.

use std::{
    cell::RefCell,
    rc::Rc,
    time::{Duration, Instant},
};

use incular_config::{Alignment, Constraints, MainAxisSize};
use incular_core::{
    Color, InputEvent, Offset, PRIMARY_POINTER_BUTTON, PointerDeviceKind, PointerPhase,
    PointerSampleMetadata, Size, Transform as Affine,
};
use incular_runtime::Runtime;
use incular_widgets::{
    Center, Directionality, GestureDetector, Positioned, RotatedBox, Row, SizedBox, Stack,
    TextDirection, Transform, Widget, internal::Key,
};

const SUBJECT: u64 = 100;
const BLUE: Color = Color::rgba(0, 0, 255, 255);
const CYAN: Color = Color::rgba(0, 255, 255, 255);

type TapLog = Rc<RefCell<Vec<&'static str>>>;

fn viewport() -> Constraints {
    Constraints::tight(Size::new(800., 600.))
}

fn subject_size(runtime: &Runtime) -> Size {
    let tree = runtime.tree();
    let element = tree
        .element_with_key(&Key::from(SUBJECT))
        .expect("fixture key");
    tree.element_bounds(element).expect("subject bounds").size
}

fn target(size: Size, color: Color, label: &'static str, log: &TapLog) -> GestureDetector {
    let log = Rc::clone(log);
    GestureDetector::new(Widget::box_(size, color)).on_tap(move || log.borrow_mut().push(label))
}

fn tap(runtime: &mut Runtime, position: Offset, id: u64, now: Instant) {
    for (phase, buttons, millis) in [
        (PointerPhase::Down, PRIMARY_POINTER_BUTTON, 0),
        (PointerPhase::Up, 0, 1),
    ] {
        let _ = runtime.handle_input_at(
            InputEvent::PointerWithMetadata {
                pointer: id,
                device: 0,
                kind: PointerDeviceKind::Touch,
                buttons,
                button: Some(PRIMARY_POINTER_BUTTON),
                sample: PointerSampleMetadata::default(),
                phase,
                position,
            },
            now + Duration::from_millis(millis),
        );
    }
    runtime
        .run_frame_at(viewport(), now + Duration::from_millis(2))
        .expect("input frame");
}

fn scaled_target_taps(
    origin: Offset,
    alignment: Option<Alignment>,
) -> (Vec<&'static str>, Vec<&'static str>) {
    let log = TapLog::default();
    let hundred = Size::new(100., 100.);
    let mut transform =
        Transform::new(Affine::scale(0.5), target(hundred, CYAN, "cyan", &log)).origin(origin);
    if let Some(alignment) = alignment {
        transform = transform.alignment(alignment);
    }
    let scene = Stack::new([
        Positioned::new(Widget::box_(hundred, BLUE))
            .top(100.)
            .left(100.),
        Positioned::new(SizedBox::square(100.).child(transform))
            .top(100.)
            .left(100.),
    ]);
    let mut runtime =
        Runtime::new(Directionality::new(TextDirection::Ltr, scene).into()).expect("mount");
    let start = Instant::now();
    runtime
        .run_frame_at(viewport(), start)
        .expect("first frame");
    tap(
        &mut runtime,
        Offset::new(110., 110.),
        1,
        start + Duration::from_millis(30),
    );
    let first = log.take();
    tap(
        &mut runtime,
        Offset::new(190., 150.),
        2,
        start + Duration::from_millis(60),
    );
    (first, log.take())
}

// widgets/transform_test.dart:19, "Transform origin".
#[test]
fn transform_hits_use_the_explicit_noncentral_origin() {
    assert_eq!(
        scaled_target_taps(Offset::new(100., 50.), None),
        (vec![], vec!["cyan"])
    );
}

// widgets/transform_test.dart:165, "Transform offset + alignment".
#[test]
fn transform_hits_use_the_sum_of_explicit_origin_and_alignment() {
    assert_eq!(
        scaled_target_taps(Offset::new(100., 0.), Some(Alignment::CENTER_LEFT)),
        (vec![], vec!["cyan"])
    );
}

// widgets/rotated_box_test.dart:9, "Rotated box control test".
#[test]
fn one_quarter_turn_swaps_the_row_extent_and_reroutes_both_taps() {
    let log = TapLog::default();
    let row = Row::new([
        target(Size::new(100., 40.), BLUE, "left", &log),
        target(Size::new(75., 65.), BLUE, "right", &log),
    ])
    .text_direction(TextDirection::Ltr)
    .main_axis_size(MainAxisSize::Min);
    let rotated = Widget::from(RotatedBox::new(1, row)).with_key(SUBJECT);
    let mut runtime = Runtime::new(Center::new(rotated).into()).expect("mount");
    let start = Instant::now();
    runtime
        .run_frame_at(viewport(), start)
        .expect("first frame");
    assert_eq!(subject_size(&runtime), Size::new(65., 175.));
    tap(
        &mut runtime,
        Offset::new(420., 280.),
        1,
        start + Duration::from_millis(30),
    );
    let left = log.take();
    tap(
        &mut runtime,
        Offset::new(380., 320.),
        2,
        start + Duration::from_millis(60),
    );
    assert_eq!((left, log.take()), (vec!["left"], vec!["right"]));
}

// widgets/rotated_box_test.dart:53, "RotatedBox does not crash at zero area".
// The required Incular child is a zero-size, nonpainting replacement for null.
#[test]
fn rotated_box_with_an_empty_child_can_complete_a_zero_area_frame() {
    let rotated = Widget::from(RotatedBox::new(1, SizedBox::shrink())).with_key(SUBJECT);
    let scene = Directionality::new(TextDirection::Ltr, Center::new(rotated));
    let mut runtime = Runtime::new(scene.into()).expect("mount");
    runtime
        .run_frame(Constraints::tight(Size::ZERO))
        .expect("zero-area frame");
    assert_eq!(subject_size(&runtime), Size::ZERO);
}

fn aligned_hit_scene(
    direction: TextDirection,
    alignment: incular_config::AlignmentGeometry,
    log: &TapLog,
) -> Widget {
    let square = Size::new(100., 100.);
    let overlay =
        Transform::new(Affine::scale(0.5), target(square, CYAN, "cyan", log)).alignment(alignment);
    Directionality::new(
        direction,
        Stack::new([
            Positioned::new(Widget::box_(square, BLUE))
                .top(100.)
                .left(100.),
            Positioned::new(SizedBox::square(100.).child(overlay))
                .top(100.)
                .left(100.),
        ]),
    )
    .into()
}

#[test]
fn physical_right_alignment_moves_the_half_scale_hit_to_the_right() {
    let log = TapLog::default();
    let mut runtime = Runtime::new(aligned_hit_scene(
        TextDirection::Ltr,
        Alignment::CENTER_RIGHT.into(),
        &log,
    ))
    .unwrap();
    let start = Instant::now();
    runtime.run_frame_at(viewport(), start).unwrap();
    tap(
        &mut runtime,
        Offset::new(110., 110.),
        1,
        start + Duration::from_millis(10),
    );
    assert!(log.take().is_empty());
    tap(
        &mut runtime,
        Offset::new(190., 150.),
        2,
        start + Duration::from_millis(20),
    );
    assert_eq!(log.take(), ["cyan"]);
}

#[test]
fn directional_alignment_changes_the_hit_pivot_on_the_same_mounted_root() {
    use incular_config::AlignmentGeometry;

    let variations = [
        (TextDirection::Ltr, AlignmentGeometry::CENTER_END, true),
        (TextDirection::Rtl, AlignmentGeometry::CENTER_START, true),
        (TextDirection::Ltr, AlignmentGeometry::CENTER_START, false),
        (TextDirection::Rtl, AlignmentGeometry::CENTER_END, false),
    ];
    let log = TapLog::default();
    let (direction, alignment, _) = variations[0];
    let mut runtime = Runtime::new(aligned_hit_scene(direction, alignment, &log)).unwrap();
    let root = runtime.tree().root().unwrap();
    let start = Instant::now();
    for (i, (direction, alignment, right)) in variations.into_iter().enumerate() {
        let now = start + Duration::from_millis(i as u64 * 40);
        runtime
            .schedule_update(root, aligned_hit_scene(direction, alignment, &log))
            .unwrap();
        runtime.run_frame_at(viewport(), now).unwrap();
        let (outside, inside) = if right {
            (Offset::new(110., 110.), Offset::new(190., 150.))
        } else {
            (Offset::new(190., 150.), Offset::new(110., 150.))
        };
        tap(
            &mut runtime,
            outside,
            i as u64 * 2 + 1,
            now + Duration::from_millis(10),
        );
        assert!(log.take().is_empty(), "{direction:?} {alignment:?}");
        tap(
            &mut runtime,
            inside,
            i as u64 * 2 + 2,
            now + Duration::from_millis(20),
        );
        assert_eq!(log.take(), ["cyan"], "{direction:?} {alignment:?}");
        assert_eq!(runtime.tree().root(), Some(root));
    }
}
