use std::{cell::RefCell, rc::Rc, time::Duration};

use incular_config::{Constraints, RuntimeEnvironment};
use incular_core::{
    Color, InputEvent, Offset, PRIMARY_POINTER_BUTTON, PointerDeviceKind, PointerPhase,
    PointerSampleMetadata, Size, time::Instant,
};
use incular_runtime::Runtime;
use incular_widgets::{Listener, Widget, internal::RawPointerEvent};

const VIEW: Size = Size {
    width: 100.,
    height: 50.,
};
const DPR: f32 = 3.;
type Trace = Rc<RefCell<Vec<RawPointerEvent>>>;

fn mount(epoch: Instant) -> (Runtime, Trace) {
    let events = Trace::default();
    let record = || {
        let events = events.clone();
        move |event| events.borrow_mut().push(event)
    };
    let child = Listener::new(Widget::box_(VIEW, Color::WHITE))
        .on_pointer_down(record())
        .on_pointer_move(record())
        .on_pointer_up(record());
    let mut runtime = Runtime::new(child.into()).unwrap();
    runtime.set_environment(RuntimeEnvironment {
        viewport: VIEW,
        physical_width: 300,
        physical_height: 150,
        scale_factor: f64::from(DPR),
        ..RuntimeEnvironment::default()
    });
    frame(&mut runtime, epoch);
    (runtime, events)
}

fn frame(runtime: &mut Runtime, now: Instant) {
    runtime.run_frame_at(Constraints::tight(VIEW), now).unwrap();
}

fn at(epoch: Instant, offset_ms: u64) -> Instant {
    epoch + Duration::from_millis(offset_ms)
}

fn send(
    runtime: &mut Runtime,
    phase: PointerPhase,
    kind: PointerDeviceKind,
    ms: Instant,
    physical_x: f32,
) {
    let down = matches!(phase, PointerPhase::Down | PointerPhase::Move);
    let _ = runtime.handle_input_at(
        InputEvent::PointerWithMetadata {
            pointer: 1,
            device: 1,
            kind,
            phase,
            position: Offset::new(physical_x / DPR, 0.),
            buttons: if down { PRIMARY_POINTER_BUTTON } else { 0 },
            button: match phase {
                PointerPhase::Up | PointerPhase::Down => Some(PRIMARY_POINTER_BUTTON),
                _ => None,
            },
            sample: PointerSampleMetadata::EMPTY,
        },
        ms,
    );
}

#[test]
fn an_opted_in_listener_receives_interpolated_frame_events_and_an_early_release() {
    let epoch = Instant::now();
    let (mut runtime, events) = mount(epoch);
    runtime.set_touch_resampling_delay(Some(Duration::from_millis(5)));
    for (ms, phase, x) in [
        (0, PointerPhase::Enter, 0.),
        (0, PointerPhase::Down, 0.),
        (10, PointerPhase::Move, 15.),
        (20, PointerPhase::Move, 30.),
        (30, PointerPhase::Move, 45.),
        (40, PointerPhase::Move, 50.),
        (40, PointerPhase::Up, 60.),
        (40, PointerPhase::Exit, 60.),
    ] {
        send(
            &mut runtime,
            phase,
            PointerDeviceKind::Touch,
            at(epoch, ms),
            x,
        );
    }
    assert!(events.borrow().is_empty());
    assert!(runtime.frame_requested());
    for (ms, count) in [(10, 1), (20, 2), (30, 4)] {
        frame(&mut runtime, at(epoch, ms));
        assert_eq!(events.borrow().len(), count);
    }
    let events = events.borrow();
    for (received, (ms, phase, x, delta)) in events.iter().zip([
        (5, PointerPhase::Down, 7.5, 0.),
        (15, PointerPhase::Move, 22.5, 15.),
        (25, PointerPhase::Move, 37.5, 15.),
        (25, PointerPhase::Up, 37.5, 0.),
    ]) {
        assert_eq!(received.time, at(epoch, ms));
        assert_eq!(received.phase, phase);
        assert_eq!(received.position, Offset::new(x / DPR, 0.));
        assert_eq!(received.delta, Offset::new(delta / DPR, 0.));
    }
    assert!(runtime.next_frame_deadline().is_none());
}

#[test]
fn stopping_resampling_flushes_queued_input_without_leaving_a_frame_deadline() {
    let epoch = Instant::now();
    let (mut runtime, events) = mount(epoch);
    runtime.set_touch_resampling_enabled(true);
    assert_eq!(
        runtime.touch_resampling_delay(),
        Some(Duration::from_millis(38))
    );
    send(
        &mut runtime,
        PointerPhase::Down,
        PointerDeviceKind::Touch,
        at(epoch, 100),
        21.,
    );
    frame(&mut runtime, at(epoch, 10));
    assert!(events.borrow().is_empty());
    assert!(runtime.next_frame_deadline().is_some());
    runtime.set_touch_resampling_enabled(false);
    assert!(runtime.touch_resampling_delay().is_none());
    assert!(runtime.next_frame_deadline().is_none());
    assert_eq!(events.borrow().len(), 1);
    assert_eq!(events.borrow()[0].time, at(epoch, 100));
    assert_eq!(events.borrow()[0].position, Offset::new(7., 0.));
    frame(&mut runtime, at(epoch, 150));
    assert_eq!(events.borrow().len(), 1);
}

#[test]
fn touches_are_immediate_by_default_and_mouse_bypasses_the_opt_in() {
    let epoch = Instant::now();
    let (mut runtime, events) = mount(epoch);
    assert!(runtime.touch_resampling_delay().is_none());
    send(
        &mut runtime,
        PointerPhase::Down,
        PointerDeviceKind::Touch,
        epoch,
        15.,
    );
    assert_eq!(events.borrow().len(), 1);
    send(
        &mut runtime,
        PointerPhase::Up,
        PointerDeviceKind::Touch,
        epoch,
        15.,
    );
    runtime.set_touch_resampling_enabled(true);
    send(
        &mut runtime,
        PointerPhase::Down,
        PointerDeviceKind::Mouse,
        at(epoch, 1),
        24.,
    );
    let events = events.borrow();
    assert_eq!(events.len(), 3);
    assert_eq!(events[2].kind, PointerDeviceKind::Mouse);
    assert_eq!(events[2].time, at(epoch, 1));
    assert_eq!(events[2].position, Offset::new(8., 0.));
}

#[test]
fn pointer_sampling_continues_on_due_ticks_without_rendering_a_frame() {
    let epoch = Instant::now();
    let (mut runtime, events) = mount(epoch);
    runtime.set_touch_resampling_delay(Some(Duration::from_millis(5)));
    for (ms, phase, x) in [
        (0, PointerPhase::Enter, 0.),
        (10, PointerPhase::Down, 0.),
        (20, PointerPhase::Move, 10.),
        (30, PointerPhase::Move, 20.),
        (40, PointerPhase::Move, 30.),
        (50, PointerPhase::Move, 40.),
        (60, PointerPhase::Move, 50.),
        (70, PointerPhase::Up, 50.),
        (70, PointerPhase::Exit, 50.),
    ] {
        send(
            &mut runtime,
            phase,
            PointerDeviceKind::Touch,
            at(epoch, ms),
            x,
        );
    }
    assert!(events.borrow().is_empty());
    assert!(runtime.frame_requested());
    frame(&mut runtime, at(epoch, 15));
    assert_eq!(events.borrow().len(), 1);
    frame(&mut runtime, at(epoch, 25));
    assert_eq!(events.borrow().len(), 2);
    assert_eq!(events.borrow()[0].time, at(epoch, 10));
    assert_eq!(events.borrow()[0].phase, PointerPhase::Down);
    assert_eq!(events.borrow()[0].position, Offset::ZERO);
    assert_eq!(events.borrow()[1].time, at(epoch, 20));
    assert_eq!(events.borrow()[1].phase, PointerPhase::Move);
    assert_eq!(events.borrow()[1].position, Offset::new(10. / DPR, 0.));
    assert_eq!(events.borrow()[1].delta, Offset::new(10. / DPR, 0.));

    let tick = Duration::from_micros(16_667);
    assert_eq!(runtime.next_frame_deadline(), Some(at(epoch, 25) + tick));
    assert!(!runtime.poll_touch_resampling_at(at(epoch, 40)));
    let rendered = runtime.diagnostics().layouts;
    assert!(runtime.poll_touch_resampling_at(at(epoch, 50)));
    assert_eq!(events.borrow().len(), 3);
    assert_eq!(events.borrow()[2].time, at(epoch, 20) + tick);
    assert_eq!(events.borrow()[2].phase, PointerPhase::Move);
    assert!(runtime.poll_touch_resampling_at(at(epoch, 67)));
    assert_eq!(events.borrow().len(), 5);
    assert_eq!(events.borrow()[3].time, at(epoch, 20) + tick * 2);
    assert_eq!(events.borrow()[3].phase, PointerPhase::Move);
    assert_eq!(events.borrow()[4].time, at(epoch, 20) + tick * 2);
    assert_eq!(events.borrow()[4].phase, PointerPhase::Up);
    assert_eq!(runtime.diagnostics().layouts, rendered);
    assert!(!runtime.poll_touch_resampling_at(at(epoch, 100)));
    assert_eq!(events.borrow().len(), 5);
}

#[test]
fn changing_the_lookbehind_does_not_rewind_an_active_contact() {
    let epoch = Instant::now();
    let (mut runtime, events) = mount(epoch);
    runtime.set_touch_resampling_delay(Some(Duration::from_millis(5)));
    for (ms, phase, x) in [
        (0, PointerPhase::Down, 0.),
        (10, PointerPhase::Move, 15.),
        (20, PointerPhase::Move, 30.),
        (30, PointerPhase::Move, 45.),
    ] {
        send(
            &mut runtime,
            phase,
            PointerDeviceKind::Touch,
            at(epoch, ms),
            x,
        );
    }
    frame(&mut runtime, at(epoch, 15));
    assert_eq!(events.borrow().len(), 1);
    assert_eq!(events.borrow()[0].position, Offset::new(5., 0.));
    runtime.set_touch_resampling_delay(Some(Duration::from_millis(20)));
    frame(&mut runtime, at(epoch, 22));
    assert_eq!(
        events.borrow().len(),
        1,
        "a bigger lookbehind would move this finger backwards"
    );
    frame(&mut runtime, at(epoch, 40));
    assert_eq!(events.borrow().len(), 2);
    assert_eq!(events.borrow()[1].position, Offset::new(10., 0.));
    assert_eq!(events.borrow()[1].delta, Offset::new(5., 0.));
}
