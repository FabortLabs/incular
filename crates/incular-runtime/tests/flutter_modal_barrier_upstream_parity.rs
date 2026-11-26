//! AnimatedModalBarrier comparisons with Flutter 3.50.0.
//! Checks modal route presence at input and settled frame checkpoints, not native-input devices or Flutter raster frames.

use incular_config::Constraints;
use incular_core::{
    Color, InputEvent, Offset, PRIMARY_POINTER_BUTTON, PointerDeviceKind, PointerPhase,
    PointerSampleMetadata, SECONDARY_POINTER_BUTTON, Size,
};
use incular_navigation::{Navigator, PageRouteBuilder, PopDecision, Route};
use incular_runtime::{RouteOutlet, Runtime};
use incular_widgets::{
    Align, Alignment, AnimatedModalBarrier, GestureDetector, HitTestBehavior, Positioned, SizedBox,
    Stack, Widget,
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
    send_pointer(
        runtime,
        at,
        pointer,
        phase,
        position,
        button,
        PointerDeviceKind::Touch,
    );
}

fn send_pointer(
    runtime: &mut Runtime,
    at: Instant,
    pointer: u64,
    phase: PointerPhase,
    position: Offset,
    button: u32,
    kind: PointerDeviceKind,
) {
    let _ = runtime.handle_input_at(
        InputEvent::PointerWithMetadata {
            pointer,
            device: 1,
            kind,
            buttons: if matches!(phase, PointerPhase::Down | PointerPhase::Move) {
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

#[derive(Clone, Copy, PartialEq, Eq)]
enum ModalScenario {
    Ordinary,
    WithDrag,
    Guarded(bool),
    Override,
}

fn check_pop(button: u32) {
    check_barrier(button, ModalScenario::Ordinary);
}

fn check_barrier(button: u32, scenario: ModalScenario) {
    let navigator = Rc::new(Navigator::new());
    let guard_calls = Rc::new(Cell::new(0));
    let dismissals = Rc::new(Cell::new(0));
    let drags = Rc::new(Cell::new(0));
    if let ModalScenario::Guarded(allow) = scenario {
        let called = guard_calls.clone();
        navigator.set_pop_guard(move |route| {
            assert_eq!(route.name, "modal");
            called.set(called.get() + 1);
            if allow {
                PopDecision::Allow
            } else {
                PopDecision::Deny
            }
        });
    }
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
            let dismissals = dismissals.clone();
            let drags = drags.clone();
            move || {
                let navigator = navigator.upgrade().expect("live navigation host");
                let barrier =
                    Widget::from(AnimatedModalBarrier::with_flat_color(RED).on_dismiss({
                        let navigator = Rc::downgrade(&navigator);
                        let dismissals = dismissals.clone();
                        move || {
                            dismissals.set(dismissals.get() + 1);
                            if scenario != ModalScenario::Override
                                && let Some(navigator) = navigator.upgrade()
                            {
                                let _ = navigator.maybe_pop();
                            }
                        }
                    }))
                    .with_key("opened modal");
                let panel = if scenario == ModalScenario::WithDrag {
                    Widget::from(Stack::new([
                        barrier,
                        Widget::from(
                            GestureDetector::new(SizedBox::new().width(800.).height(600.))
                                .behavior(HitTestBehavior::Translucent)
                                .on_vertical_drag_start({
                                    let drags = drags.clone();
                                    move |_| drags.set(drags.get() + 1)
                                }),
                        ),
                    ]))
                } else {
                    barrier
                };
                navigator.push(PageRouteBuilder::new("modal", panel).build());
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
    assert_eq!((guard_calls.get(), dismissals.get()), (0, 0));

    if scenario == ModalScenario::WithDrag {
        for (ms, phase, offset) in [
            (1034, PointerPhase::Down, 0.),
            (1036, PointerPhase::Move, 65.),
            (1038, PointerPhase::Up, 65.),
        ] {
            send_touch(
                &mut runtime,
                now + Duration::from_millis(ms),
                3,
                phase,
                BARRIER_CENTER + Offset::new(0., offset),
                PRIMARY_POINTER_BUTTON,
            );
        }
        present(&mut runtime, 1040);
        assert_eq!(snapshot(&runtime), ("modal".to_owned(), true));
        assert_eq!((dismissals.get(), drags.get()), (0, 1));

        for (ms, phase) in [(1042, PointerPhase::Down), (1043, PointerPhase::Cancel)] {
            send_touch(
                &mut runtime,
                now + Duration::from_millis(ms),
                4,
                phase,
                BARRIER_CENTER,
                PRIMARY_POINTER_BUTTON,
            );
        }
        present(&mut runtime, 1044);
        assert_eq!(snapshot(&runtime), ("modal".to_owned(), true));
        assert_eq!(dismissals.get(), 0);

        for (ms, phase, offset) in [
            (1045, PointerPhase::Down, 0.),
            (1046, PointerPhase::Move, 65.),
            (1047, PointerPhase::Up, 65.),
        ] {
            send_pointer(
                &mut runtime,
                now + Duration::from_millis(ms),
                5,
                phase,
                BARRIER_CENTER + Offset::new(0., offset),
                SECONDARY_POINTER_BUTTON,
                PointerDeviceKind::Mouse,
            );
        }
        assert_eq!(dismissals.get(), 0, "a nonprimary mouse drag is not a tap");
        assert_eq!(drags.get(), 1);
    }

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
    let remains = matches!(
        scenario,
        ModalScenario::Guarded(false) | ModalScenario::Override
    );
    assert_eq!(
        snapshot(&runtime),
        (
            if remains { "modal" } else { "initial" }.to_owned(),
            remains
        ),
        "release only pops a permitted route with the default dismiss handler"
    );
    assert_eq!(dismissals.get(), 1);
    assert_eq!(
        guard_calls.get(),
        usize::from(matches!(scenario, ModalScenario::Guarded(_)))
    );
    assert_eq!(
        drags.get(),
        usize::from(scenario == ModalScenario::WithDrag)
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

#[test]
fn additional_buttons_release_the_modal_barrier_without_dismissing_on_press() {
    for button in [
        incular_core::TERTIARY_POINTER_BUTTON,
        incular_core::BACK_POINTER_BUTTON,
        incular_core::FORWARD_POINTER_BUTTON,
        SECONDARY_POINTER_BUTTON | incular_core::TERTIARY_POINTER_BUTTON,
    ] {
        check_pop(button);
    }
}

#[test]
fn a_drag_or_cancel_leaves_the_barrier_open_but_a_tap_with_a_drag_competitor_pops_it() {
    check_barrier(PRIMARY_POINTER_BUTTON, ModalScenario::WithDrag);
}

#[test]
fn the_barrier_consults_the_pop_guard_on_release_and_respects_both_results() {
    for allow in [false, true] {
        check_barrier(PRIMARY_POINTER_BUTTON, ModalScenario::Guarded(allow));
    }
}

#[test]
fn an_override_receives_dismissal_instead_of_popping_the_route() {
    check_barrier(PRIMARY_POINTER_BUTTON, ModalScenario::Override);
}
