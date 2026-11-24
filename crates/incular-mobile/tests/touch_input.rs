//! Native touch and mouse samples reach the runtime with the button state
//! and the platform time the shell reports. No GPU is needed: frames run
//! through the application directly, as the desktop input tests do.

use incular_config::Constraints;
use incular_core::{
    Color, Offset, PRIMARY_POINTER_BUTTON, PointerDeviceKind, PointerPhase,
    SECONDARY_POINTER_BUTTON, Size,
};
use incular_mobile::{MobileEngine, ScrollSample, TouchSample};
use incular_platform::PhysicalSize;
use incular_runtime::{Application, RuntimeWake};
use incular_widgets::{
    DragEndDetails, GestureDetector, HitTestBehavior, Listener, MouseRegion, ScrollController,
    SingleChildScrollView, Widget,
};
use std::{
    cell::RefCell,
    rc::Rc,
    sync::Arc,
    time::{Duration, Instant},
};

struct NoWake;

impl RuntimeWake for NoWake {
    fn wake(&self) {}
}

fn engine(root: Widget, scale: u32) -> MobileEngine {
    let application = Application::new(move |_| root.clone()).expect("application");
    let mut engine = MobileEngine::new(application, Arc::new(NoWake));
    engine.surface_changed(
        PhysicalSize::new(100 * scale, 400 * scale),
        f64::from(scale),
    );
    let id = engine.window_id().expect("the first window is bound");
    let frame = engine
        .application_mut()
        .run_window_frame_at(
            id,
            Constraints::tight(Size::new(100., 400.)),
            Instant::now(),
        )
        .expect("frame");
    assert!(frame.is_some(), "the window should produce a layout frame");
    engine
}

fn sample(
    kind: PointerDeviceKind,
    phase: PointerPhase,
    buttons: u32,
    button: Option<u32>,
    position: Offset,
    age: Duration,
) -> TouchSample {
    TouchSample {
        pointer: 0,
        device: 0,
        phase,
        kind,
        x: f64::from(position.x),
        y: f64::from(position.y),
        pressure: None,
        buttons,
        button,
        stylus: None,
        age,
    }
}

#[test]
fn mouse_secondary_button_press_and_release_reach_the_pointer_listener() {
    let seen = Rc::new(RefCell::new(Vec::new()));
    let root = Listener::new(Widget::box_(Size::new(100., 400.), Color::WHITE))
        .behavior(HitTestBehavior::Opaque)
        .on_pointer_down({
            let seen = seen.clone();
            move |event| {
                seen.borrow_mut()
                    .push((event.device, event.buttons, event.button))
            }
        })
        .on_pointer_up({
            let seen = seen.clone();
            move |event| {
                seen.borrow_mut()
                    .push((event.device, event.buttons, event.button))
            }
        });
    let mut engine = engine(root.into(), 1);

    let mut mouse = sample(
        PointerDeviceKind::Mouse,
        PointerPhase::Down,
        SECONDARY_POINTER_BUTTON,
        Some(SECONDARY_POINTER_BUTTON),
        Offset::new(10., 10.),
        Duration::ZERO,
    );
    mouse.device = 42;
    engine.touch(mouse);
    mouse.phase = PointerPhase::Up;
    mouse.buttons = 0;
    engine.touch(mouse);

    assert_eq!(
        &*seen.borrow(),
        &[
            (42, SECONDARY_POINTER_BUTTON, Some(SECONDARY_POINTER_BUTTON)),
            (42, 0, Some(SECONDARY_POINTER_BUTTON)),
        ]
    );
}

#[test]
fn mouse_hover_notifies_the_region_until_the_pointer_exits() {
    let seen = Rc::new(RefCell::new(Vec::new()));
    let root = MouseRegion::new(Widget::box_(Size::new(100., 400.), Color::WHITE))
        .on_enter({
            let seen = seen.clone();
            move |event| seen.borrow_mut().push(("enter", event.position))
        })
        .on_hover({
            let seen = seen.clone();
            move |event| seen.borrow_mut().push(("hover", event.position))
        })
        .on_exit({
            let seen = seen.clone();
            move |event| seen.borrow_mut().push(("exit", event.position))
        });
    let mut engine = engine(root.into(), 2);

    let point = Offset::new(20., 40.);
    for phase in [PointerPhase::Enter, PointerPhase::Move, PointerPhase::Exit] {
        engine.touch(sample(
            PointerDeviceKind::Mouse,
            phase,
            0,
            None,
            point,
            Duration::ZERO,
        ));
    }

    assert_eq!(
        *seen.borrow(),
        ["enter", "hover", "exit"].map(|callback| (callback, Offset::new(10., 20.))),
    );
}

#[test]
fn wheel_hits_its_viewport_without_sending_a_pointer_move() {
    let controller = ScrollController::new();
    let moves = Rc::new(RefCell::new(Vec::new()));
    let list = SingleChildScrollView::new(Widget::box_(Size::new(100., 1000.), Color::WHITE))
        .controller(controller.clone());
    let root = Listener::new(list)
        .behavior(HitTestBehavior::Opaque)
        .on_pointer_hover({
            let moves = moves.clone();
            move |event| moves.borrow_mut().push(event.position)
        });
    let mut engine = engine(root.into(), 2);

    engine.touch(sample(
        PointerDeviceKind::Mouse,
        PointerPhase::Move,
        0,
        None,
        Offset::new(20., 20.),
        Duration::ZERO,
    ));

    let mut wheel = ScrollSample {
        x: 210.,
        y: 40.,
        delta_x: 0.,
        delta_y: 80.,
        age: Duration::ZERO,
    };
    engine.scroll(wheel);
    assert_eq!(controller.offset(), 0.);

    wheel.x = 20.;
    engine.scroll(wheel);
    assert_eq!(controller.offset(), 40.);
    assert_eq!(*moves.borrow(), [Offset::new(10., 10.)]);
}

#[test]
fn batched_touch_samples_keep_their_platform_spacing() {
    // Four samples 16 ms apart, delivered back to back as Android batches a
    // MotionEvent's history. The drag rises 2 px/ms, so the release fling
    // must read about -2000 px/s, not the delivery burst's arbitrary speed.
    let release = Rc::new(RefCell::new(None::<DragEndDetails>));
    let root = GestureDetector::new(Widget::box_(Size::new(100., 400.), Color::WHITE))
        .behavior(HitTestBehavior::Opaque)
        .on_pan_end({
            let release = release.clone();
            move |details| *release.borrow_mut() = Some(details)
        });
    let mut engine = engine(root.into(), 1);

    let at = |age_ms: u64| {
        // Ages count down to the release, so the first sample is oldest.
        let elapsed_ms = 48 - age_ms as i64;
        let y = 300. - 2. * elapsed_ms as f32;
        (Offset::new(50., y), Duration::from_millis(age_ms))
    };
    let (position, age) = at(48);
    engine.touch(sample(
        PointerDeviceKind::Touch,
        PointerPhase::Down,
        PRIMARY_POINTER_BUTTON,
        Some(PRIMARY_POINTER_BUTTON),
        position,
        age,
    ));
    for age_ms in [32, 16] {
        let (position, age) = at(age_ms);
        engine.touch(sample(
            PointerDeviceKind::Touch,
            PointerPhase::Move,
            PRIMARY_POINTER_BUTTON,
            None,
            position,
            age,
        ));
    }
    let (position, age) = at(0);
    engine.touch(sample(
        PointerDeviceKind::Touch,
        PointerPhase::Up,
        0,
        Some(PRIMARY_POINTER_BUTTON),
        position,
        age,
    ));

    let details = release
        .borrow_mut()
        .take()
        .expect("the release ends the pan");
    let velocity = details.velocity.pixels_per_second;
    assert!(
        (velocity.y + 2000.).abs() < 200.,
        "release velocity {velocity:?} should be about -2000 px/s"
    );
}
