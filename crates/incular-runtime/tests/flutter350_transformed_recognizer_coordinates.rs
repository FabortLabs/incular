//! Flutter 3.50.0 transformed tap, long-press, and scale callbacks.

use std::{cell::RefCell, rc::Rc, time::Duration};

use incular_config::Constraints;
use incular_core::{
    Color, InputEvent, Offset, PRIMARY_POINTER_BUTTON, PointerDeviceKind, PointerPhase,
    PointerSampleMetadata, Size, time::Instant,
};
use incular_runtime::Runtime;
use incular_widgets::{Center, GestureDetector, Transform, Widget};

type Positions = (Offset, Offset);

#[derive(Default)]
struct Recording {
    down: Vec<Positions>,
    up: Vec<Positions>,
    tap: u32,
    cancel: u32,
}

const LOCAL: Offset = Offset::new(50., 75.);
const GLOBAL: Offset = Offset::new(400., 300.);

fn check(scale: Option<f32>) {
    let record = Rc::new(RefCell::new(Recording::default()));
    let detector = GestureDetector::new(Widget::box_(
        Size::new(100., 150.),
        Color::rgba(255, 0, 0, 255),
    ))
    .on_tap_down({
        let record = record.clone();
        move |d| {
            record
                .borrow_mut()
                .down
                .push((d.local_position, d.global_position))
        }
    })
    .on_tap_up({
        let record = record.clone();
        move |d| {
            record
                .borrow_mut()
                .up
                .push((d.local_position, d.global_position))
        }
    })
    .on_tap({
        let record = record.clone();
        move || record.borrow_mut().tap += 1
    })
    .on_tap_cancel({
        let record = record.clone();
        move || record.borrow_mut().cancel += 1
    });
    let widget: Widget = match scale {
        Some(scale) => Transform::scale(scale, detector).into(),
        None => detector.into(),
    };
    let mut runtime = Runtime::new(Center::new(widget).into()).unwrap();
    let now = Instant::now();
    runtime
        .run_frame_at(Constraints::tight(Size::new(800., 600.)), now)
        .unwrap();

    let mut sequence = |pointer, shift, start| {
        for (phase, y, millis) in [
            (PointerPhase::Down, 300., start),
            (PointerPhase::Move, 300. + shift, start + 1),
            (PointerPhase::Up, 300. + shift, start + 2),
        ] {
            touch(
                &mut runtime,
                now + Duration::from_millis(millis),
                pointer,
                phase,
                Offset::new(400., y),
            );
        }
    };

    let shift = scale.map_or(0., |_| 17.);
    sequence(1, shift, 0);
    {
        let first = record.take();
        assert_eq!((first.tap, first.cancel), (1, 0));
        assert_eq!(first.down, [(LOCAL, GLOBAL)]);
        assert_eq!(
            first.up,
            [(
                LOCAL + Offset::new(0., shift / scale.unwrap_or(1.)),
                GLOBAL + Offset::new(0., shift),
            )]
        );
    }
    if scale.is_some() {
        sequence(2, 19., 30);
        let second = record.take();
        assert_eq!((second.tap, second.cancel), (0, 1));
        assert_eq!(second.down, [(LOCAL, GLOBAL)]);
        assert!(second.up.is_empty());
    }
}

#[test]
fn tap_reports_local_and_global_positions_in_a_centered_detector() {
    check(None);
}

#[test]
fn scaling_up_does_not_scale_the_global_tap_slop() {
    check(Some(2.));
}

#[test]
fn scaling_down_does_not_scale_the_global_tap_slop() {
    check(Some(0.5));
}

fn touch(
    runtime: &mut Runtime,
    time: Instant,
    pointer: u64,
    phase: PointerPhase,
    position: Offset,
) {
    let _ = runtime.handle_input_at(
        InputEvent::PointerWithMetadata {
            pointer,
            device: 1,
            kind: PointerDeviceKind::Touch,
            button: (phase != PointerPhase::Move).then_some(PRIMARY_POINTER_BUTTON),
            phase,
            buttons: if phase == PointerPhase::Up {
                0
            } else {
                PRIMARY_POINTER_BUTTON
            },
            position,
            sample: PointerSampleMetadata::EMPTY,
        },
        time,
    );
}

#[derive(Default)]
struct LongTrace {
    start: Vec<Positions>,
    end: Vec<Positions>,
    moves: Vec<(Positions, Positions)>,
    press_count: u32,
    up_count: u32,
}

fn long_press_positions(scale: Option<f32>) {
    let trace = Rc::new(RefCell::new(LongTrace::default()));
    let detector = GestureDetector::new(Widget::box_(
        Size::new(100., 150.),
        Color::rgba(255, 0, 0, 255),
    ))
    .on_long_press_start({
        let trace = trace.clone();
        move |d| {
            trace
                .borrow_mut()
                .start
                .push((d.local_position, d.global_position))
        }
    })
    .on_long_press_end({
        let trace = trace.clone();
        move |d| {
            trace
                .borrow_mut()
                .end
                .push((d.local_position, d.global_position))
        }
    })
    .on_long_press_move_update({
        let trace = trace.clone();
        move |d| {
            trace.borrow_mut().moves.push((
                (d.local_position, d.global_position),
                (d.local_offset_from_origin, d.offset_from_origin),
            ))
        }
    })
    .on_long_press({
        let trace = trace.clone();
        move || trace.borrow_mut().press_count += 1
    })
    .on_long_press_up({
        let trace = trace.clone();
        move || trace.borrow_mut().up_count += 1
    });
    let widget: Widget = match scale {
        Some(scale) => Transform::scale(scale, detector).into(),
        None => detector.into(),
    };
    let mut runtime = Runtime::new(Center::new(widget).into()).unwrap();
    let now = Instant::now();
    let view = Constraints::tight(Size::new(800., 600.));
    runtime.run_frame_at(view, now).unwrap();

    let mut sequence = |pointer, millis, before, after| {
        let start = now + Duration::from_millis(millis);
        let deadline = start + Duration::from_millis(500);
        touch(&mut runtime, start, pointer, PointerPhase::Down, GLOBAL);
        if before != 0. {
            touch(
                &mut runtime,
                start + Duration::from_millis(1),
                pointer,
                PointerPhase::Move,
                GLOBAL + Offset::new(0., before),
            );
        }
        runtime.run_frame_at(view, deadline).unwrap();
        if after != 0. {
            touch(
                &mut runtime,
                deadline + Duration::from_millis(1),
                pointer,
                PointerPhase::Move,
                GLOBAL + Offset::new(0., after),
            );
        }
        touch(
            &mut runtime,
            deadline + Duration::from_millis(2),
            pointer,
            PointerPhase::Up,
            GLOBAL + Offset::new(0., before + after),
        );
    };
    let position = |distance| {
        (
            LOCAL + Offset::new(0., distance / scale.unwrap_or(1.)),
            GLOBAL + Offset::new(0., distance),
        )
    };

    let before = scale.map_or(0., |_| 10.);
    sequence(1, 0, before, 0.);
    let first = trace.take();
    assert_eq!((first.press_count, first.up_count), (1, 1));
    assert_eq!(first.start, [(LOCAL, GLOBAL)]);
    assert_eq!(first.end, [position(before)]);
    assert!(first.moves.is_empty());
    if let Some(scale) = scale {
        sequence(2, 600, 0., 100.);
        let second = trace.take();
        assert_eq!((second.press_count, second.up_count), (1, 1));
        assert_eq!(second.start, [(LOCAL, GLOBAL)]);
        assert_eq!(second.end, [position(100.)]);
        assert_eq!(
            second.moves,
            [(
                position(100.),
                (Offset::new(0., 100. / scale), Offset::new(0., 100.))
            )]
        );
    }
}

#[test]
fn centered_long_press_reports_local_and_global_start_and_end() {
    long_press_positions(None);
}

#[test]
fn scaled_up_long_press_reports_local_deltas_only_after_recognition() {
    long_press_positions(Some(2.));
}

#[test]
fn scaled_down_long_press_reports_local_deltas_only_after_recognition() {
    long_press_positions(Some(0.5));
}

#[test]
fn two_pointer_scale_reports_the_local_and_global_focal_points() {
    let starts = Rc::new(RefCell::new(Vec::new()));
    let updates = Rc::new(RefCell::new(Vec::new()));
    let detector = GestureDetector::new(Widget::box_(
        Size::new(100., 100.),
        Color::rgba(255, 0, 0, 255),
    ))
    .on_scale_start({
        let starts = starts.clone();
        move |d| {
            starts
                .borrow_mut()
                .push((d.local_focal_point, d.focal_point))
        }
    })
    .on_scale_update({
        let updates = updates.clone();
        move |d| {
            updates
                .borrow_mut()
                .push((d.local_focal_point, d.focal_point))
        }
    });
    let now = Instant::now();
    let mut runtime = Runtime::new(Center::new(detector).into()).unwrap();
    let view = Constraints::tight(Size::new(800., 600.));
    runtime.run_frame_at(view, now).unwrap();
    touch(
        &mut runtime,
        now,
        1,
        PointerPhase::Down,
        Offset::new(380., 280.),
    );
    touch(
        &mut runtime,
        now + Duration::from_millis(1),
        2,
        PointerPhase::Down,
        Offset::new(430., 330.),
    );
    touch(
        &mut runtime,
        now + Duration::from_millis(2),
        2,
        PointerPhase::Move,
        Offset::new(420., 320.),
    );
    assert_eq!(
        updates.borrow().as_slice(),
        [(Offset::new(50., 50.), GLOBAL)]
    );
    assert_eq!(
        starts.borrow().as_slice(),
        [
            (Offset::new(30., 30.), Offset::new(380., 280.)),
            (Offset::new(50., 50.), GLOBAL),
        ]
    );
    touch(
        &mut runtime,
        now + Duration::from_millis(3),
        1,
        PointerPhase::Up,
        Offset::new(380., 280.),
    );
    touch(
        &mut runtime,
        now + Duration::from_millis(4),
        2,
        PointerPhase::Up,
        Offset::new(420., 320.),
    );
}
