use std::{
    cell::RefCell,
    rc::Rc,
    time::{Duration, Instant},
};

use incular_config::{Alignment, Axis, Constraints};
use incular_core::{Color, Offset, PointerPhase, Size, Transform as Affine};
use incular_gestures::PointerEvent;
use incular_widgets::{
    Center, DragDownDetails, DragEndDetails, DragStartDetails, DragUpdateDetails, GestureDetector,
    Transform, Widget, internal::WidgetTree,
};

const ORIGIN: Offset = Offset { x: 400., y: 300. };
const LOCAL: Offset = Offset { x: 50., y: 75. };

#[derive(Default)]
struct Trace {
    cancelled: usize,
    down: Vec<DragDownDetails>,
    start: Vec<DragStartDetails>,
    update: Vec<DragUpdateDetails>,
    end: Vec<DragEndDetails>,
}

fn drag(axis: Axis, affine: Affine, competes: bool, steps: &[Offset]) -> Trace {
    let trace = Rc::new(RefCell::new(Trace::default()));
    macro_rules! record {
        ($field:ident) => {{
            let trace = trace.clone();
            move |event| trace.borrow_mut().$field.push(event)
        }};
    }
    let cancel = {
        let trace = trace.clone();
        move || trace.borrow_mut().cancelled += 1
    };
    let gesture = GestureDetector::new(Widget::box_(
        Size::new(100., 150.),
        Color::rgba(255, 0, 0, 255),
    ));
    let mut gesture = match axis {
        Axis::Horizontal => gesture
            .on_horizontal_drag_down(record!(down))
            .on_horizontal_drag_start(record!(start))
            .on_horizontal_drag_update(record!(update))
            .on_horizontal_drag_end(record!(end))
            .on_horizontal_drag_cancel(cancel),
        Axis::Vertical => gesture
            .on_vertical_drag_down(record!(down))
            .on_vertical_drag_start(record!(start))
            .on_vertical_drag_update(record!(update))
            .on_vertical_drag_end(record!(end))
            .on_vertical_drag_cancel(cancel),
    };
    if competes {
        gesture = gesture.on_tap(|| {});
    }
    let mut tree = WidgetTree::new();
    tree.mount(Center::new(Transform::new(affine, gesture).alignment(Alignment::CENTER)).into())
        .unwrap();
    tree.layout(Constraints::tight(Size::new(800., 600.)))
        .unwrap();
    let base = Instant::now();
    let mut pos = ORIGIN;
    let mut send = |phase, position, n| {
        let _ = tree.dispatch_gesture(PointerEvent {
            pointer: 1,
            position,
            phase,
            time: base + Duration::from_millis(n * 10),
        });
    };
    send(PointerPhase::Down, pos, 0);
    for (i, step) in steps.iter().enumerate() {
        pos = pos + *step;
        send(PointerPhase::Move, pos, (i + 1) as u64);
    }
    send(PointerPhase::Up, pos, steps.len() as u64 + 1);
    std::mem::take(&mut *trace.borrow_mut())
}

fn close(position: Offset, expected: Offset, tolerance: f32) {
    let d = position - expected;
    assert!(d.x.hypot(d.y) <= tolerance, "{position:?} != {expected:?}");
}

fn check_down(trace: &Trace) {
    assert_eq!(trace.down.len(), 1);
    close(trace.down[0].global_position, ORIGIN, 0.0001);
    close(trace.down[0].local_position, LOCAL, 0.0001);
}

fn check_untransformed(axis: Axis) {
    let step = axis.offset(100., 0.);
    let trace = drag(axis, Affine::IDENTITY, false, &[step]);
    check_down(&trace);
    assert_eq!(trace.cancelled, 0);
    assert_eq!(trace.end.len(), 1);
    assert_eq!(trace.start.len(), 1);
    assert_eq!(trace.start[0].global_position, ORIGIN);
    assert_eq!(trace.start[0].local_position, LOCAL);
    let last = trace.update.last().unwrap();
    assert_eq!(last.global_position, ORIGIN + step);
    assert_eq!(last.local_position, LOCAL + step);
    assert_eq!(
        trace
            .update
            .iter()
            .fold(Offset::ZERO, |total, update| total + update.delta),
        step
    );
    assert_eq!(
        trace
            .update
            .iter()
            .map(|update| update.primary_delta.unwrap_or(0.))
            .sum::<f32>(),
        100.
    );
}

#[test]
fn horizontal_gets_the_exact_local_coordinates() {
    check_untransformed(Axis::Horizontal);
}

#[test]
fn vertical_gets_the_exact_local_coordinates() {
    check_untransformed(Axis::Vertical);
}

fn check_scale(axis: Axis, scale: f32) {
    let affine = Affine::scale(scale);
    let crossing = drag(axis, affine, true, &[axis.offset(19., 0.)]);
    check_down(&crossing);
    assert_eq!(crossing.cancelled, 0);
    assert_eq!(crossing.end.len(), 1);
    assert_eq!(crossing.start.len(), 1);
    assert_eq!(
        crossing.start[0].local_position,
        LOCAL + axis.offset(19. / scale, 0.)
    );
    assert_eq!(
        crossing.start[0].global_position,
        ORIGIN + axis.offset(19., 0.)
    );
    assert!(crossing.update.is_empty());

    let inside = drag(axis, affine, true, &[axis.offset(17., 0.)]);
    check_down(&inside);
    assert!(inside.start.is_empty() && inside.update.is_empty() && inside.end.is_empty());

    let first = axis.offset(19., 30.);
    let second = axis.offset(100., 10.);
    let two = drag(axis, affine, true, &[first, second]);
    check_down(&two);
    assert_eq!(two.cancelled, 0);
    assert_eq!(two.start.len(), 1);
    assert_eq!(two.update.len(), 1);
    assert_eq!(two.end.len(), 1);
    assert_eq!(two.start[0].global_position, ORIGIN + first);
    assert_eq!(
        two.start[0].local_position,
        LOCAL + Offset::new(first.x / scale, first.y / scale)
    );
    assert_eq!(two.update[0].global_position, ORIGIN + first + second);
    assert_eq!(
        two.update[0].local_position,
        LOCAL + Offset::new((first.x + second.x) / scale, (first.y + second.y) / scale)
    );
    assert_eq!(two.update[0].delta, axis.offset(100. / scale, 0.));
    assert_eq!(two.update[0].primary_delta, Some(100. / scale));
    assert_eq!(inside.cancelled, 1, "the losing drag reports one cancel");
}

#[test]
fn scaled_up_horizontal_uses_global_slop() {
    check_scale(Axis::Horizontal, 2.);
}

#[test]
fn scaled_down_horizontal_uses_global_slop() {
    check_scale(Axis::Horizontal, 0.5);
}

#[test]
fn scaled_up_vertical_uses_global_slop() {
    check_scale(Axis::Vertical, 2.);
}

#[test]
fn scaled_down_vertical_uses_global_slop() {
    check_scale(Axis::Vertical, 0.5);
}

#[test]
fn rotated_horizontal_uses_global_length_of_the_local_axis() {
    let affine = Affine::rotation(std::f32::consts::FRAC_PI_4);
    let inside = drag(Axis::Horizontal, affine, true, &[Offset::new(9., 9.)]);
    check_down(&inside);
    assert!(inside.start.is_empty() && inside.update.is_empty() && inside.end.is_empty());

    let two = drag(
        Axis::Horizontal,
        affine,
        true,
        &[Offset::new(18., 18.), Offset::new(3., 4.)],
    );
    check_down(&two);
    assert_eq!(two.cancelled, 0);
    assert_eq!(two.start.len(), 1);
    assert_eq!(two.update.len(), 1);
    assert_eq!(two.end.len(), 1);
    close(
        two.update[0].global_position,
        Offset::new(421., 322.),
        0.0001,
    );
    close(two.update[0].delta, Offset::new(5., 0.), 0.1);
    assert!((two.update[0].primary_delta.unwrap() - 5.).abs() < 0.1);
    assert_eq!(inside.cancelled, 1, "the losing drag reports one cancel");
}
