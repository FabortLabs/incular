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
    Align, Alignment, AnimatedModalBarrier, GestureDetector, HitTestBehavior, ModalBarrier,
    Positioned, SizedBox, Stack, Widget,
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum BarrierKind {
    Static,
    Animated,
}

impl BarrierKind {
    fn plain(self, dismissible: bool) -> Widget {
        match self {
            Self::Static => ModalBarrier::new().dismissible(dismissible).into(),
            Self::Animated => AnimatedModalBarrier::with_flat_color(RED)
                .dismissible(dismissible)
                .into(),
        }
    }
}

fn check_stack_tap(target_in_front: bool, kind: BarrierKind) {
    let taps = Rc::new(Cell::new(0));
    let target = Widget::from(
        GestureDetector::new(Widget::box_(Size::new(10., 10.), Color::WHITE)).on_tap({
            let taps = taps.clone();
            move || taps.set(taps.get() + 1)
        }),
    )
    .with_key("tap target");
    let barrier = Widget::from(Positioned::fill(kind.plain(false))).with_key("barrier");
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
    for kind in [BarrierKind::Static, BarrierKind::Animated] {
        check_stack_tap(false, kind);
    }
}

/// modal_barrier_test.dart:556 "AnimatedModalBarrier does not prevent interactions with widgets in front of it".
#[test]
fn a_non_dismissible_barrier_allows_taps_in_front_of_it() {
    for kind in [BarrierKind::Static, BarrierKind::Animated] {
        check_stack_tap(true, kind);
    }
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
    check_barrier_kind(button, scenario, BarrierKind::Animated);
}

fn check_barrier_kind(button: u32, scenario: ModalScenario, kind: BarrierKind) {
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
                let barrier: Widget = if kind == BarrierKind::Static {
                    let mut barrier = ModalBarrier::new();
                    if scenario == ModalScenario::Override {
                        let dismissals = dismissals.clone();
                        barrier = barrier.on_dismiss(move || {
                            dismissals.set(dismissals.get() + 1);
                        });
                    }
                    // No handler in the other static cases: use the
                    // production nearest-outlet default.
                    barrier.into()
                } else {
                    AnimatedModalBarrier::with_flat_color(RED)
                        .on_dismiss({
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
                        })
                        .into()
                };
                let barrier = barrier.with_key("opened modal");
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
    assert_eq!(
        dismissals.get(),
        usize::from(kind == BarrierKind::Animated || scenario == ModalScenario::Override)
    );
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

#[test]
fn static_barrier_primary_and_secondary_release_pop_the_nearest_outlet() {
    for button in [PRIMARY_POINTER_BUTTON, SECONDARY_POINTER_BUTTON] {
        check_barrier_kind(button, ModalScenario::Ordinary, BarrierKind::Static);
    }
}

#[test]
fn static_barrier_uses_the_shared_drag_arena_and_pop_guard() {
    for scenario in [
        ModalScenario::WithDrag,
        ModalScenario::Guarded(false),
        ModalScenario::Guarded(true),
        ModalScenario::Override,
    ] {
        check_barrier_kind(PRIMARY_POINTER_BUTTON, scenario, BarrierKind::Static);
    }
}

#[test]
fn locked_modal_without_a_label_advertises_no_semantic_node() {
    for kind in [BarrierKind::Static, BarrierKind::Animated] {
        let mut runtime = Runtime::new(kind.plain(false)).unwrap();
        runtime.run_frame(bounds()).unwrap();
        assert_eq!(runtime.tree().semantics().iter().count(), 0, "{kind:?}");
    }
}

#[test]
fn labeled_barrier_exposes_and_dispatches_both_activation_and_dismissal() {
    for kind in [BarrierKind::Static, BarrierKind::Animated] {
        check_labeled_barrier_actions(kind);
    }
}

fn check_labeled_barrier_actions(kind: BarrierKind) {
    use incular_core::Rect;
    use incular_semantics::{SemanticAction, SemanticActionKind, SemanticRole};
    use incular_widgets::{Directionality, TextDirection};
    let events = Rc::new(Cell::new(0));
    let build = |dismissible| {
        let events = events.clone();
        let callback = move || events.set(events.get() + 1);
        let barrier: Widget = match kind {
            BarrierKind::Static => ModalBarrier::new()
                .semantics_label("Dismiss")
                .dismissible(dismissible)
                .on_dismiss(callback)
                .into(),
            BarrierKind::Animated => AnimatedModalBarrier::with_flat_color(RED)
                .semantics_label("Dismiss")
                .dismissible(dismissible)
                .on_dismiss(callback)
                .into(),
        };
        Directionality::new(TextDirection::Ltr, barrier).into()
    };
    let mut runtime = Runtime::new(build(true)).unwrap();
    runtime.run_frame(bounds()).unwrap();
    let find_label = |runtime: &Runtime| {
        runtime
            .tree()
            .semantics()
            .iter()
            .find_map(|(id, node)| (node.label.as_deref() == Some("Dismiss")).then_some(id))
            .expect("labeled barrier node")
    };
    let id = find_label(&runtime);
    let data = runtime.tree().semantics().node(id).unwrap();
    assert_eq!(data.role, SemanticRole::Button);
    assert_eq!(data.state.text_direction, Some(TextDirection::Ltr));
    assert_eq!(
        data.bounds,
        Rect::from_origin_size(Offset::ZERO, Size::new(800., 600.))
    );
    assert_eq!(
        data.actions,
        [SemanticActionKind::Activate, SemanticActionKind::Dismiss]
    );
    assert!(runtime.dispatch_semantic_action(id, SemanticAction::Activate));
    assert!(runtime.dispatch_semantic_action(id, SemanticAction::Dismiss));
    assert_eq!(events.get(), 2);

    let root = runtime.tree().root().unwrap();
    runtime.schedule_update(root, build(false)).unwrap();
    runtime.run_frame(bounds()).unwrap();
    let locked = find_label(&runtime);
    assert!(
        runtime
            .tree()
            .semantics()
            .node(locked)
            .unwrap()
            .actions
            .is_empty()
    );
    assert!(!runtime.dispatch_semantic_action(locked, SemanticAction::Dismiss));
    assert!(!runtime.dispatch_semantic_action(id, SemanticAction::Dismiss));
    assert_eq!(events.get(), 2);
}

#[test]
fn static_and_animated_barriers_can_lay_out_at_zero_area() {
    for kind in [BarrierKind::Static, BarrierKind::Animated] {
        check_barrier_at_zero_area(kind);
    }
}

fn check_barrier_at_zero_area(kind: BarrierKind) {
    let child = kind.plain(true).with_key("zero-barrier");
    let scene =
        incular_widgets::Center::new(SizedBox::from_dimensions(Some(0.), Some(0.), Some(child)));
    let mut runtime = Runtime::new(scene.into()).unwrap();
    runtime.run_frame(bounds()).unwrap();
    let target = runtime
        .tree()
        .element_with_key(&"zero-barrier".into())
        .unwrap();
    assert_eq!(
        runtime.tree().element_bounds(target).unwrap().size,
        Size::ZERO
    );
}

#[test]
fn a_translucent_drag_in_front_of_a_locked_barrier_still_reaches_its_recognizer() {
    for kind in [BarrierKind::Static, BarrierKind::Animated] {
        check_translucent_drag_in_front_of_barrier(kind);
    }
}

fn check_translucent_drag_in_front_of_barrier(kind: BarrierKind) {
    let moves = Rc::new(Cell::new(0));
    let target = GestureDetector::new(incular_widgets::Center::new(Widget::box_(
        Size::new(50., 30.),
        Color::WHITE,
    )))
    .behavior(HitTestBehavior::Translucent)
    .on_horizontal_drag_start({
        let moves = moves.clone();
        move |_| moves.set(moves.get() + 1)
    });
    let scene = Stack::new([kind.plain(false), Widget::from(target)]);
    let mut runtime = Runtime::new(scene.into()).unwrap();
    let now = Instant::now();
    runtime.run_frame_at(bounds(), now).unwrap();
    for (ms, phase, point) in [
        (2, PointerPhase::Down, Offset::new(790., 590.)),
        (4, PointerPhase::Move, Offset::new(770., 590.)),
        (6, PointerPhase::Up, Offset::new(770., 590.)),
    ] {
        send_touch(
            &mut runtime,
            now + Duration::from_millis(ms),
            1,
            phase,
            point,
            PRIMARY_POINTER_BUTTON,
        );
    }
    assert_eq!(moves.get(), 1);
}

fn mouse(runtime: &mut Runtime, at: Instant, point: Offset, phase: PointerPhase) {
    let _ = runtime.handle_input_at(
        InputEvent::PointerWithMetadata {
            pointer: 41,
            device: 0,
            kind: PointerDeviceKind::Mouse,
            buttons: u32::from(phase == PointerPhase::Down),
            button: matches!(phase, PointerPhase::Down | PointerPhase::Up)
                .then_some(PRIMARY_POINTER_BUTTON),
            sample: PointerSampleMetadata::default(),
            phase,
            position: point,
        },
        at,
    );
}

#[test]
fn the_locked_barrier_blocks_hover_behind_it_but_not_in_front() {
    for kind in [BarrierKind::Static, BarrierKind::Animated] {
        check_locked_barrier_hover(kind);
    }
}

fn check_locked_barrier_hover(kind: BarrierKind) {
    use incular_widgets::MouseRegion;
    for front in [false, true] {
        let log = Rc::new(RefCell::new(Vec::new()));
        let region = MouseRegion::new(Widget::box_(Size::new(10., 10.), Color::WHITE))
            .on_enter({
                let log = log.clone();
                move |_| log.borrow_mut().push("enter")
            })
            .on_exit({
                let log = log.clone();
                move |_| log.borrow_mut().push("exit")
            });
        let subject = Widget::from(Positioned::new(region).left(0.).top(0.));
        let locked = kind.plain(false);
        let stack = Stack::new(if front {
            [locked, subject]
        } else {
            [subject, locked]
        });
        let mut runtime = Runtime::new(stack.into()).unwrap();
        let now = Instant::now();
        runtime.run_frame_at(bounds(), now).unwrap();
        mouse(
            &mut runtime,
            now + Duration::from_millis(2),
            Offset::new(100., 100.),
            PointerPhase::Move,
        );
        assert!(log.borrow().is_empty());
        if front {
            mouse(
                &mut runtime,
                now + Duration::from_millis(3),
                ORIGIN_TARGET,
                PointerPhase::Move,
            );
            assert_eq!(*log.borrow(), ["enter"]);
        } else {
            mouse(
                &mut runtime,
                now + Duration::from_millis(3),
                ORIGIN_TARGET,
                PointerPhase::Down,
            );
            mouse(
                &mut runtime,
                now + Duration::from_millis(4),
                ORIGIN_TARGET,
                PointerPhase::Up,
            );
            assert!(log.borrow().is_empty());
        }
        mouse(
            &mut runtime,
            now + Duration::from_millis(5),
            Offset::new(100., 100.),
            PointerPhase::Move,
        );
        assert_eq!(
            &*log.borrow(),
            if front {
                &["enter", "exit"][..]
            } else {
                &[][..]
            }
        );
    }
}

#[test]
fn a_modal_barrier_keeps_the_basic_cursor_over_a_click_cursor() {
    use incular_widgets::{MouseCursor, MouseRegion};
    for dismissible in [false, true] {
        let underneath = MouseRegion::new(Widget::box_(Size::new(800., 600.), Color::WHITE))
            .cursor(MouseCursor::Click);
        let barrier = ModalBarrier::new().dismissible(dismissible);
        let scene = Stack::new([Widget::from(underneath), Widget::from(barrier)]);
        let mut runtime = Runtime::new(scene.into()).unwrap();
        let now = Instant::now();
        runtime.run_frame_at(bounds(), now).unwrap();
        mouse(
            &mut runtime,
            now + Duration::from_millis(2),
            BARRIER_CENTER,
            PointerPhase::Enter,
        );
        assert_eq!(
            runtime.effective_mouse_cursor(),
            MouseCursor::Basic,
            "dismissible: {dismissible}"
        );
    }
}

#[test]
fn a_barrier_uses_the_current_handler_when_an_active_tap_releases() {
    use incular_core::ValueNotifier;
    use incular_widgets::Builder;
    for button in [PRIMARY_POINTER_BUTTON, SECONDARY_POINTER_BUTTON] {
        let generation = ValueNotifier::new(1);
        let callbacks = Rc::new(RefCell::new(Vec::new()));
        let mut runtime = Runtime::new(
            Builder::new({
                let generation = generation.clone();
                let callbacks = callbacks.clone();
                move |_| {
                    let value = generation.value();
                    let callbacks = callbacks.clone();
                    AnimatedModalBarrier::with_flat_color(RED)
                        .on_dismiss(move || callbacks.borrow_mut().push(value))
                        .into()
                }
            })
            .into(),
        )
        .unwrap();
        let now = Instant::now();
        runtime.run_frame_at(bounds(), now).unwrap();
        send_touch(
            &mut runtime,
            now + Duration::from_millis(2),
            1,
            PointerPhase::Down,
            BARRIER_CENTER,
            button,
        );
        generation.set_value(2);
        runtime
            .run_frame_at(bounds(), now + Duration::from_millis(4))
            .unwrap();
        assert!(callbacks.borrow().is_empty());
        send_touch(
            &mut runtime,
            now + Duration::from_millis(6),
            1,
            PointerPhase::Up,
            BARRIER_CENTER,
            button,
        );
        assert_eq!(&*callbacks.borrow(), &[2]);
    }
}

#[test]
fn reactive_barrier_insets_clip_only_the_semantic_rectangle() {
    use incular_config::EdgeInsets;
    use incular_core::{Rect, ValueNotifier};
    use incular_semantics::{SemanticAction, SemanticActionKind};

    fn rect(left: f32, top: f32, right: f32, bottom: f32) -> Rect {
        Rect::from_origin_size(
            Offset::new(left, top),
            Size::new(right - left, bottom - top),
        )
    }

    let original = ValueNotifier::new(EdgeInsets::from_ltrb(10., 20., 30., 40.));
    let replacement = ValueNotifier::new(EdgeInsets::from_ltrb(8., 9., 10., 11.));
    let calls = Rc::new(Cell::new(0));
    let build = |source: ValueNotifier<EdgeInsets>| {
        let calls = calls.clone();
        Stack::new([Widget::from(
            ModalBarrier::new()
                .semantics_label("Dismiss")
                .clip_details_notifier(source)
                .on_dismiss(move || calls.set(calls.get() + 1)),
        )])
        .into()
    };
    let mut runtime = Runtime::new(build(original.clone())).unwrap();
    let now = Instant::now();
    let (before, _) = runtime.run_frame_at(bounds(), now).unwrap();
    let id = runtime
        .tree()
        .semantics()
        .iter()
        .find_map(|(id, node)| (node.label.as_deref() == Some("Dismiss")).then_some(id))
        .unwrap();
    let node = runtime.tree().semantics().node(id).unwrap();
    assert_eq!(node.bounds, rect(10., 20., 770., 560.));
    assert_eq!(
        node.actions,
        [SemanticActionKind::Activate, SemanticActionKind::Dismiss]
    );

    let point = Offset::new(12., 24.);
    assert_eq!(
        runtime
            .tree()
            .semantic_action_target_at(point, &SemanticAction::Dismiss),
        Some(id)
    );
    original.set_value(EdgeInsets::from_ltrb(40., 30., 20., 10.));
    assert!(runtime.frame_requested());
    // Input and bounds agree with the last completed semantics frame.
    assert_eq!(
        runtime
            .tree()
            .semantic_action_target_at(point, &SemanticAction::Dismiss),
        Some(id)
    );
    let (after, frame) = runtime
        .run_frame_at(bounds(), now + Duration::from_millis(16))
        .unwrap();
    assert_eq!(frame.rebuilt_elements, 0);
    assert_eq!(frame.laid_out_render_objects, 0);
    assert_eq!(frame.repainted_render_objects, 0);
    assert_eq!(before, after);
    assert_eq!(
        runtime.tree().semantics().node(id).unwrap().bounds,
        rect(40., 30., 780., 590.)
    );
    assert!(!runtime.dispatch_semantic_action_at(point, SemanticAction::Dismiss));
    assert!(!runtime.dispatch_semantic_action_at(Offset::new(780., 60.), SemanticAction::Dismiss));
    assert!(runtime.dispatch_semantic_action_at(Offset::new(40., 30.), SemanticAction::Dismiss));
    assert_eq!(calls.get(), 1);

    for (phase, millis) in [(PointerPhase::Down, 20), (PointerPhase::Up, 22)] {
        send_touch(
            &mut runtime,
            now + Duration::from_millis(millis),
            1,
            phase,
            point,
            PRIMARY_POINTER_BUTTON,
        );
    }
    assert_eq!(calls.get(), 2, "physical taps still reach the full veil");

    runtime
        .schedule_update(runtime.tree().root().unwrap(), build(replacement.clone()))
        .unwrap();
    runtime
        .run_frame_at(bounds(), now + Duration::from_millis(32))
        .unwrap();
    assert_eq!(
        runtime.tree().semantics().node(id).unwrap().bounds,
        rect(8., 9., 790., 589.)
    );
    assert!(!runtime.frame_requested());
    original.set_value(EdgeInsets::all(60.));
    assert!(!runtime.frame_requested(), "old notifier was detached");
    replacement.set_value(EdgeInsets::ZERO);
    assert!(runtime.frame_requested());
    runtime
        .run_frame_at(bounds(), now + Duration::from_millis(48))
        .unwrap();
    assert_eq!(
        runtime.tree().semantics().node(id).unwrap().bounds,
        rect(0., 0., 800., 600.)
    );

    let excluded: Widget = Stack::new([Widget::from(
        ModalBarrier::new()
            .semantics_label("Dismiss")
            .clip_details_notifier(replacement.clone())
            .barrier_semantics_dismissible(false),
    )])
    .into();
    let root = runtime.tree().root().unwrap();
    runtime.schedule_update(root, excluded).unwrap();
    runtime
        .run_frame_at(bounds(), now + Duration::from_millis(64))
        .unwrap();
    let node = runtime.tree().semantics().node(id).unwrap();
    assert_eq!(node.bounds, rect(0., 0., 800., 600.));
    assert!(node.actions.is_empty());
    replacement.set_value(EdgeInsets::all(100.));
    assert!(
        !runtime.frame_requested(),
        "excluded semantics are not tracked"
    );

    runtime
        .schedule_update(root, build(replacement.clone()))
        .unwrap();
    runtime
        .run_frame_at(bounds(), now + Duration::from_millis(80))
        .unwrap();
    assert!(runtime.tree().semantics().iter().any(|(_, node)| {
        node.label.as_deref() == Some("Dismiss") && node.bounds == rect(100., 100., 700., 500.)
    }));
    runtime
        .schedule_update(root, Stack::new([Widget::box_(Size::ZERO, RED)]).into())
        .unwrap();
    runtime
        .run_frame_at(bounds(), now + Duration::from_millis(96))
        .unwrap();
    replacement.set_value(EdgeInsets::all(120.));
    assert!(
        !runtime.frame_requested(),
        "unmounted barriers are not tracked"
    );
}

#[test]
fn static_barrier_default_is_unpainted_and_explicit_color_paints_the_full_veil() {
    use incular_core::Rect;
    use incular_rendering::{Brush, PaintCommand};
    let mut runtime = Runtime::new(ModalBarrier::new().into()).unwrap();
    let (no_fill, stats) = runtime.run_frame(bounds()).unwrap();
    assert!(!no_fill.commands().iter().any(|command| {
        matches!(
            command,
            PaintCommand::Rect { .. } | PaintCommand::RRect { .. }
        )
    }));
    assert_eq!(stats.active_animations, 0);

    let root = runtime.tree().root().unwrap();
    runtime
        .schedule_update(root, ModalBarrier::new().color(RED).into())
        .unwrap();
    let (colored, stats) = runtime.run_frame(bounds()).unwrap();
    let expected = Rect::from_origin_size(Offset::ZERO, Size::new(800., 600.));
    assert!(
        colored.commands().iter().any(|command| match command {
            PaintCommand::Rect { rect, color } => *rect == expected && *color == RED,
            PaintCommand::RRect {
                rrect,
                brush: Brush::Solid(color),
            } => {
                rrect.rect == expected && rrect.radii == Default::default() && *color == RED
            }
            _ => false,
        }),
        "unexpected veil paint: {colored:?}"
    );
    assert!(
        !colored
            .commands()
            .iter()
            .any(|command| matches!(command, PaintCommand::Border { .. }))
    );
    assert_eq!(stats.active_animations, 0);
}
