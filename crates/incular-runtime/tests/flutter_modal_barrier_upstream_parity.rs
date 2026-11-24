//! AnimatedModalBarrier comparisons with Flutter 3.50.0.
//! No semantics, transition timing, or native-input device behavior is implied.

use incular_config::Constraints;
use incular_core::{
    Color, InputEvent, Offset, PRIMARY_POINTER_BUTTON, PointerDeviceKind, PointerPhase,
    PointerSampleMetadata, SECONDARY_POINTER_BUTTON, Size,
};
use incular_navigation::{ModalBarrier, Navigator, Route};
use incular_runtime::{RouteOutlet, Runtime};
use incular_widgets::{
    Align, Alignment, AnimatedModalBarrier, GestureDetector, Positioned, SizedBox, Stack, Widget,
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::{Duration, Instant},
};

const RED: Color = Color::rgba(255, 0, 0, 255);
const ORIGIN_TARGET: Offset = Offset::new(5., 5.);
const BARRIER_CENTER: Offset = Offset::new(400., 300.);

fn bounds() -> Constraints {
    Constraints::tight(Size::new(800., 600.))
}

fn send_touch(
    runtime: &mut Runtime,
    at: Instant,
    pointer: u64,
    phase: PointerPhase,
    position: Offset,
    button: u32,
) {
    // Flutter WidgetController.press defaults to touch. Its non-primary
    // test sets the secondary bit without switching to a native mouse.
    let _ = runtime.handle_input_at(
        InputEvent::PointerWithMetadata {
            pointer,
            device: 1,
            kind: PointerDeviceKind::Touch,
            buttons: if phase == PointerPhase::Down {
                button
            } else {
                0
            },
            button: Some(button),
            sample: PointerSampleMetadata::default(),
            phase,
            position,
        },
        at,
    );
}

fn check_stack_tap(target_in_front: bool) {
    let taps = Rc::new(Cell::new(0));
    let target = Widget::from(
        GestureDetector::new(Widget::box_(Size::new(10., 10.), Color::WHITE)).on_tap({
            let taps = taps.clone();
            move || taps.set(taps.get() + 1)
        }),
    )
    .with_key("tap target");
    let barrier = Widget::from(Positioned::fill(
        AnimatedModalBarrier::with_flat_color(RED).dismissible(false),
    ))
    .with_key("barrier");
    let children = if target_in_front {
        [barrier, target]
    } else {
        [target, barrier]
    };
    let mut runtime = Runtime::new(Stack::new(children).into()).expect("mount stack");
    let now = Instant::now();
    runtime.run_frame_at(bounds(), now).expect("first frame");
    assert!(
        runtime
            .tree()
            .element_with_key(&"tap target".into())
            .is_some()
    );
    assert!(runtime.tree().element_with_key(&"barrier".into()).is_some());
    for phase in [PointerPhase::Down, PointerPhase::Up] {
        send_touch(
            &mut runtime,
            now,
            1,
            phase,
            ORIGIN_TARGET,
            PRIMARY_POINTER_BUTTON,
        );
    }
    runtime
        .run_frame_at(bounds(), now + Duration::from_millis(16))
        .expect("tap frame");
    assert_eq!(taps.get(), u32::from(target_in_front));
}

/// modal_barrier_test.dart:510 "AnimatedModalBarrier prevents interactions with widgets behind it".
#[test]
fn a_non_dismissible_barrier_blocks_taps_behind_it() {
    check_stack_tap(false);
}

/// modal_barrier_test.dart:556 "AnimatedModalBarrier does not prevent interactions with widgets in front of it".
#[test]
fn a_non_dismissible_barrier_allows_taps_in_front_of_it() {
    check_stack_tap(true);
}

fn check_pop(button: u32) {
    let navigator = Rc::new(Navigator::new());
    let mut runtime = Runtime::new(SizedBox::shrink().into()).expect("mount navigation host");
    let outlet = Rc::new(RefCell::new(RouteOutlet::new(
        &navigator,
        &runtime.spawner().scope(),
    )));
    let root = runtime.tree().root().expect("root outlet host");
    runtime
        .register_builder(root, {
            let outlet = outlet.clone();
            move || {
                SizedBox::from_dimensions(Some(800.), Some(600.), Some(outlet.borrow().widget()))
                    .into()
            }
        })
        .expect("register outlet host");
    let now = Instant::now();
    runtime
        .run_frame_at(bounds(), now)
        .expect("build outlet host");
    RouteOutlet::attach(&outlet, &mut runtime).expect("attach public outlet");

    let start_button = GestureDetector::new(Widget::box_(Size::new(10., 10.), Color::WHITE))
        .on_tap({
            let navigator = Rc::downgrade(&navigator);
            move || {
                let navigator = navigator.upgrade().expect("live navigation host");
                let marker = Widget::from(SizedBox::shrink()).with_key("opened modal");
                navigator.push(Route::new("modal", marker).modal(ModalBarrier {
                    color: RED,
                    ..ModalBarrier::default()
                }));
            }
        });
    navigator.push(Route::new(
        "initial",
        Align::new(Alignment::TOP_LEFT, start_button),
    ));
    let present = |runtime: &mut Runtime, elapsed| {
        RouteOutlet::present_frame_at(
            &outlet,
            runtime,
            bounds(),
            now + Duration::from_millis(elapsed),
        )
        .expect("present mounted route");
    };
    let snapshot = |runtime: &Runtime| {
        (
            navigator.current().expect("initial route remains").name,
            runtime
                .tree()
                .element_with_key(&"opened modal".into())
                .is_some(),
        )
    };
    present(&mut runtime, 16);
    assert_eq!(snapshot(&runtime), ("initial".to_owned(), false));

    for phase in [PointerPhase::Down, PointerPhase::Up] {
        send_touch(
            &mut runtime,
            now + Duration::from_millis(24),
            1,
            phase,
            ORIGIN_TARGET,
            PRIMARY_POINTER_BUTTON,
        );
    }
    present(&mut runtime, 32);
    present(&mut runtime, 1032);
    assert_eq!(snapshot(&runtime), ("modal".to_owned(), true));

    send_touch(
        &mut runtime,
        now + Duration::from_millis(1048),
        2,
        PointerPhase::Down,
        BARRIER_CENTER,
        button,
    );
    present(&mut runtime, 1064);
    assert_eq!(
        snapshot(&runtime),
        ("modal".to_owned(), true),
        "press alone does not pop"
    );

    send_touch(
        &mut runtime,
        now + Duration::from_millis(1080),
        2,
        PointerPhase::Up,
        BARRIER_CENTER,
        button,
    );
    present(&mut runtime, 1096);
    present(&mut runtime, 2096);
    assert_eq!(
        snapshot(&runtime),
        ("initial".to_owned(), false),
        "release pops the modal"
    );
}

/// modal_barrier_test.dart:665 "AnimatedModalBarrier pops the Navigator when dismissed by primary tap".
#[test]
fn primary_release_on_the_barrier_pops_the_modal_route() {
    check_pop(PRIMARY_POINTER_BUTTON);
}

/// modal_barrier_test.dart:696 "AnimatedModalBarrier pops the Navigator when dismissed by non-primary tap".
#[test]
fn secondary_release_on_the_barrier_pops_the_modal_route() {
    check_pop(SECONDARY_POINTER_BUTTON);
}
