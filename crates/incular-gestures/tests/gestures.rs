use incular_core::{Code, KeyboardEvent, Modifiers, Offset, PointerPhase, Rect};
use incular_gestures::*;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::{Duration, Instant},
};

#[test]
fn shortcuts_only_dispatch_matching_pressed_events() {
    let count = Rc::new(Cell::new(0));
    let mut shortcuts = Shortcuts::new();
    let expected = count.clone();
    shortcuts.register(
        ShortcutKey::new(Code::KeyA, Modifiers::default()),
        move || expected.set(expected.get() + 1),
    );
    assert!(shortcuts.handle(KeyboardEvent::key_down(
        incular_core::KeyboardKey::Named(incular_core::NamedKey::Unidentified),
        Code::KeyA,
    )));
    assert_eq!(count.get(), 1);
}
#[test]
fn focus_manager_keeps_one_owner_and_skips_excluded_nodes() {
    let first = FocusNode::new();
    let skipped = FocusNode::new();
    let last = FocusNode::new();
    skipped.set_can_request_focus(false);
    let mut scope = FocusManager::new();
    for node in [&first, &skipped, &last] {
        scope.register(node);
    }
    assert!(scope.request_focus(&first));
    assert!(first.has_focus());
    assert_eq!(scope.focus_next(false), Some(last.clone()));
    assert!(!first.has_focus());
    assert!(last.has_focus());
    assert_eq!(scope.focus_next(true), Some(first.clone()));
    drop(last);
    assert_eq!(scope.registered_count(), 2);
}

#[test]
fn focus_manager_applies_reading_and_explicit_order_policies() {
    let top_right = FocusNode::new();
    let bottom_left = FocusNode::new();
    let top_left = FocusNode::new();
    top_right.set_rect(Rect::from_origin_size(
        Offset::new(100., 0.),
        incular_core::Size::new(20., 20.),
    ));
    bottom_left.set_rect(Rect::from_origin_size(
        Offset::new(0., 50.),
        incular_core::Size::new(20., 20.),
    ));
    top_left.set_rect(Rect::from_origin_size(
        Offset::new(0., 0.),
        incular_core::Size::new(20., 20.),
    ));
    let mut reading = FocusManager::with_policy(FocusTraversalPolicyKind::ReadingOrder);
    for node in [&top_right, &bottom_left, &top_left] {
        reading.register(node);
    }
    assert_eq!(reading.focus_next(false), Some(top_left.clone()));
    assert_eq!(reading.focus_next(false), Some(top_right.clone()));
    assert_eq!(reading.focus_next(false), Some(bottom_left.clone()));

    let first = FocusNode::new();
    let second = FocusNode::new();
    first.set_traversal_order(Some(20.));
    second.set_traversal_order(Some(10.));
    let mut ordered = FocusManager::with_policy(FocusTraversalPolicyKind::Ordered);
    ordered.register(&first);
    ordered.register(&second);
    assert_eq!(ordered.focus_next(false), Some(second));
    assert_eq!(ordered.focus_next(false), Some(first));
}
#[test]
fn shortcuts_can_dispatch_typed_commands() {
    let count = Rc::new(Cell::new(0));
    let mut actions = Actions::new();
    actions.register("save", {
        let count = count.clone();
        move || count.set(count.get() + 1)
    });
    let mut shortcuts = Shortcuts::new();
    shortcuts.bind(ShortcutKey::new(Code::KeyA, Modifiers::default()), "save");
    assert!(shortcuts.handle_actions(
        KeyboardEvent::key_down(
            incular_core::KeyboardKey::Named(incular_core::NamedKey::Unidentified),
            Code::KeyA,
        ),
        &actions,
    ));
    assert_eq!(count.get(), 1);
}
#[test]
fn generic_commands_dispatch_ctrl_s_without_string_conversion() {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    enum EditorCommand {
        Save,
    }

    let count = Rc::new(Cell::new(0));
    let mut actions = Actions::<EditorCommand>::typed();
    let observed = count.clone();
    actions.register(EditorCommand::Save, move || {
        observed.set(observed.get() + 1)
    });
    let mut shortcuts = Shortcuts::<EditorCommand>::typed();
    shortcuts.bind(
        ShortcutKey::new(Code::KeyS, Modifiers::CONTROL),
        EditorCommand::Save,
    );

    let mut event = KeyboardEvent::key_down(
        incular_core::KeyboardKey::Named(incular_core::NamedKey::Unidentified),
        Code::KeyS,
    );
    event.modifiers = Modifiers::CONTROL;
    assert!(shortcuts.handle_actions(event, &actions));
    assert_eq!(count.get(), 1);
}
#[test]
fn actions_dispatch_nearest_scope_first() {
    let observed = Rc::new(RefCell::new(Vec::new()));
    let outer_observed = observed.clone();
    let mut actions = Actions::new();
    actions.register("open", move || outer_observed.borrow_mut().push("outer"));
    actions.push_scope();
    let inner_observed = observed.clone();
    actions.register("open", move || inner_observed.borrow_mut().push("inner"));

    assert!(actions.invoke(&Command::new("open")));
    assert_eq!(&*observed.borrow(), &["inner"]);
}

#[test]
fn action_invocation_listeners_observe_start_and_end() {
    let phases = Rc::new(RefCell::new(Vec::new()));
    let action = Action::<Command>::new("save", || {});
    let observed = phases.clone();
    let _subscription = action.add_invocation_listener(move |phase| {
        observed.borrow_mut().push(phase);
    });

    assert_eq!(
        action.invoke(&Intent::<Command>::from("save")),
        ActionResult::Handled
    );
    assert_eq!(
        &*phases.borrow(),
        &[ActionInvocationPhase::Start, ActionInvocationPhase::End]
    );
}

#[test]
fn disabled_or_ignored_actions_fall_through_to_outer_scope() {
    let observed = Rc::new(RefCell::new(Vec::new()));
    let outer_observed = observed.clone();
    let mut actions = Actions::new();
    actions.register("save", move || outer_observed.borrow_mut().push("outer"));
    actions.push_scope();
    let disabled = Action::new("save", {
        let observed = observed.clone();
        move || observed.borrow_mut().push("disabled")
    });
    disabled.set_enabled(false);
    actions.register_action(disabled);
    assert!(actions.invoke(&Command::new("save")));
    assert_eq!(&*observed.borrow(), &["outer"]);

    actions.register_handler("save", |_| ActionResult::Ignored);
    assert!(actions.invoke(&Command::new("save")));
    assert_eq!(&*observed.borrow(), &["outer", "outer"]);

    actions.register_handler("save", |_| false);
    assert!(actions.invoke(&Command::new("save")));
    assert_eq!(&*observed.borrow(), &["outer", "outer", "outer"]);

    let ignored = Action::<Command>::with_handler("save", |_| false);
    assert_eq!(
        ignored.invoke(&Intent::<Command>::from("save")),
        ActionResult::Ignored
    );
}

#[test]
fn temporary_action_scope_restores_after_callback() {
    let observed = Rc::new(RefCell::new(Vec::new()));
    let outer_observed = observed.clone();
    let mut actions = Actions::new();
    actions.register("toggle", move || outer_observed.borrow_mut().push("outer"));
    actions.with_scope(|actions| {
        let inner_observed = observed.clone();
        actions.register("toggle", move || inner_observed.borrow_mut().push("inner"));
        assert_eq!(actions.scope_depth(), 2);
        assert!(actions.invoke(&Command::new("toggle")));
    });
    assert_eq!(actions.scope_depth(), 1);
    assert!(actions.invoke(&Command::new("toggle")));
    assert_eq!(&*observed.borrow(), &["inner", "outer"]);
}

#[test]
fn shortcuts_distinguish_press_repeat_and_release() {
    let presses = Rc::new(Cell::new(0));
    let repeats = Rc::new(Cell::new(0));
    let releases = Rc::new(Cell::new(0));
    let mut shortcuts = Shortcuts::new();
    let observed_presses = presses.clone();
    shortcuts.register_on(
        ShortcutKey::new(Code::KeyA, Modifiers::default()),
        ShortcutTrigger::Press,
        move || observed_presses.set(observed_presses.get() + 1),
    );
    let observed_repeats = repeats.clone();
    shortcuts.register_on(
        ShortcutKey::new(Code::KeyA, Modifiers::default()),
        ShortcutTrigger::Repeat,
        move || observed_repeats.set(observed_repeats.get() + 1),
    );
    let observed_releases = releases.clone();
    shortcuts.register_on(
        ShortcutKey::new(Code::KeyA, Modifiers::default()),
        ShortcutTrigger::Up,
        move || observed_releases.set(observed_releases.get() + 1),
    );

    assert!(shortcuts.handle(KeyboardEvent::key_down(
        incular_core::KeyboardKey::Named(incular_core::NamedKey::Unidentified),
        Code::KeyA,
    )));
    let mut repeat = KeyboardEvent::key_down(
        incular_core::KeyboardKey::Named(incular_core::NamedKey::Unidentified),
        Code::KeyA,
    );
    repeat.repeat = true;
    assert!(shortcuts.handle(repeat));
    assert!(shortcuts.handle(KeyboardEvent::key_up(
        incular_core::KeyboardKey::Named(incular_core::NamedKey::Unidentified),
        Code::KeyA,
    )));
    assert_eq!(presses.get(), 1);
    assert_eq!(repeats.get(), 1);
    assert_eq!(releases.get(), 1);
}

#[test]
fn logical_shortcut_and_nested_scope_fallthrough_are_supported() {
    let observed = Rc::new(Cell::new(0));
    let mut actions = Actions::new();
    let outer = observed.clone();
    actions.register("save", move || outer.set(outer.get() + 1));

    let mut shortcuts = Shortcuts::new();
    shortcuts.bind_logical(
        LogicalShortcutKey::new(
            incular_core::KeyboardKey::Character("s".into()),
            Modifiers::CONTROL,
        ),
        "save",
    );
    shortcuts.push_scope();
    shortcuts.bind_on(
        ShortcutKey::new(Code::KeyS, Modifiers::CONTROL),
        ShortcutTrigger::Press,
        Intent::new("unhandled"),
    );

    let mut event =
        KeyboardEvent::key_down(incular_core::KeyboardKey::Character("s".into()), Code::KeyS);
    event.modifiers = Modifiers::CONTROL;
    assert!(shortcuts.handle_actions(event, &actions));
    assert_eq!(observed.get(), 1);
}

#[test]
fn focus_scope_node_restores_previous_focus() {
    let scope = FocusScopeNode::new();
    let first = FocusNode::new();
    let second = FocusNode::new();
    scope.register(&first);
    scope.register(&second);
    assert!(scope.request_focus(&first));
    scope.clear_focus();
    assert!(scope.focused().is_none());
    assert!(scope.restore_focus());
    assert_eq!(scope.focused(), Some(first));
    assert_eq!(scope.focus_next(false), Some(second.clone()));
    assert_eq!(scope.focused(), Some(second));
}

#[test]
fn focus_scope_observer_tracks_revision_and_stops_after_drop() {
    let scope = FocusScopeNode::new();
    let node = FocusNode::new();
    let notifications = Rc::new(Cell::new(0));
    let observed = notifications.clone();
    let subscription = scope.observe(move || observed.set(observed.get() + 1));
    let initial_revision = scope.revision();

    scope.register(&node);
    assert!(scope.revision() > initial_revision);
    assert_eq!(notifications.get(), 1);
    assert!(scope.request_focus(&node));
    assert_eq!(notifications.get(), 2);

    drop(subscription);
    scope.clear_focus();
    assert_eq!(notifications.get(), 2);
    assert!(scope.revision() > initial_revision);
}

#[test]
fn nested_focus_scope_restores_parent_child() {
    let parent = FocusScopeNode::new();
    let parent_child = FocusNode::new();
    parent.register(&parent_child);
    assert!(parent.request_focus(&parent_child));

    let child = FocusScopeNode::nested(&parent);
    let child_node = FocusNode::new();
    child.register(&child_node);
    assert!(child.request_focus(&child_node));
    assert!(parent.focused().is_none());
    child.unfocus();
    assert!(child.restore_parent_focus());
    assert_eq!(parent.focused(), Some(parent_child));
}
#[test]
fn scale_recognizer_reports_relative_two_pointer_distance() {
    let observed = Rc::new(Cell::new(0.));
    let expected = observed.clone();
    let mut detector = ScaleGestureDetector::new(move |details| expected.set(details.scale));
    let now = Instant::now();
    for (pointer, position) in [(1, Offset::new(0., 0.)), (2, Offset::new(10., 0.))] {
        assert!(detector.handle(PointerEvent {
            pointer,
            position,
            phase: PointerPhase::Down,
            time: now,
        }));
    }
    assert!(detector.handle(PointerEvent {
        pointer: 2,
        position: Offset::new(20., 0.),
        phase: PointerPhase::Move,
        time: now,
    }));
    assert_eq!(observed.get(), 2.);
}

#[test]
fn arena_rejects_competing_drag_but_keeps_compatible_scale_members() {
    let key = GestureArenaKey::pointer(7, 3);
    let mut arena = GestureArena::new();
    let tap = arena.add(key, false);
    let drag = arena.add(key, false);
    let entries = arena.accept(key, drag);
    assert!(
        entries
            .iter()
            .any(|entry| entry.member == drag && entry.disposition == GestureDisposition::Accepted)
    );
    assert!(
        entries
            .iter()
            .any(|entry| entry.member == tap && entry.disposition == GestureDisposition::Rejected)
    );
    let scale_a = arena.add(key, true);
    let scale_b = arena.add(key, true);
    let entries = arena.accept(key, scale_a);
    assert!(
        entries.iter().any(
            |entry| entry.member == scale_b && entry.disposition == GestureDisposition::Pending
        )
    );
    assert_eq!(arena.cancel(key).len(), 4);
}

#[test]
fn arena_keys_keep_pointer_devices_and_trackpad_streams_disjoint() {
    let mouse = GestureArenaKey::pointer_device(7, 11, 3);
    let pen = GestureArenaKey::pointer_device(7, 22, 3);
    let pinch = GestureArenaKey::trackpad(7, 11, incular_core::TrackpadGestureKind::Pinch);

    assert_ne!(
        mouse, pen,
        "equal contact IDs on different devices are distinct"
    );
    assert_ne!(
        mouse, pinch,
        "aggregate gestures never alias pointer contacts"
    );
    assert_eq!(mouse.pointer_id(), Some(3));
    assert_eq!(mouse.pointer_device_id(), Some(11));
    assert_eq!(pinch.pointer_id(), None);
}
fn drag_event(position: Offset, phase: PointerPhase, base: Instant, millis: u64) -> PointerEvent {
    PointerEvent {
        pointer: 1,
        position,
        phase,
        time: base + Duration::from_millis(millis),
    }
}

/// Feeds a move and unwraps the accepted action.
fn accepted(recognizer: &mut PointerGestureRecognizer, event: PointerEvent) -> GestureAction {
    match recognizer.observe(event) {
        GestureDecision::Accept(action) => action,
        other => panic!("expected an accepted action, got {other:?}"),
    }
}

#[test]
fn drag_recognizer_reports_start_updates_and_end() {
    // Flutter 3.41 gestures/monodrag.dart: a PanGestureRecognizer accepts once
    // the path length exceeds computePanSlop (36 for touch/unknown), starts at
    // the accepted position (DragStartBehavior.start) and reports each later
    // move as `delta`, the movement since the previous event.
    let starts = Rc::new(RefCell::new(Vec::new()));
    let updates = Rc::new(RefCell::new(Vec::new()));
    let ends = Rc::new(RefCell::new(Vec::new()));
    let mut detector = DragGestureDetector::new(DragCallbacks {
        on_start: Some({
            let starts = starts.clone();
            Rc::new(move |details| starts.borrow_mut().push(details))
        }),
        on_update: Some({
            let updates = updates.clone();
            Rc::new(move |details| updates.borrow_mut().push(details))
        }),
        on_end: Some({
            let ends = ends.clone();
            Rc::new(move |details| ends.borrow_mut().push(details))
        }),
    });
    let now = Instant::now();
    for (phase, position, millis) in [
        (PointerPhase::Down, Offset::ZERO, 0),
        // Under the slop: nothing is reported.
        (PointerPhase::Move, Offset::new(20., 0.), 10),
        // Path length 40 > 36: accepted here, with no update for this move.
        (PointerPhase::Move, Offset::new(40., 0.), 20),
        (PointerPhase::Move, Offset::new(55., 5.), 30),
        (PointerPhase::Up, Offset::new(55., 5.), 40),
    ] {
        assert!(detector.handle(drag_event(position, phase, now, millis)));
    }
    let starts = starts.borrow();
    assert_eq!(starts.len(), 1);
    assert_eq!(
        starts[0].global_position,
        Offset::new(40., 0.),
        "the drag starts where it was accepted, not at the pointer down"
    );
    assert_eq!(starts[0].kind, PointerDeviceKind::Unknown);
    let updates = updates.borrow();
    assert_eq!(updates.len(), 1, "the accepting move is not an update");
    assert_eq!(updates[0].delta, Offset::new(15., 5.));
    assert_eq!(updates[0].primary_delta, None, "a pan has no primary axis");
    assert_eq!(updates[0].global_position, Offset::new(55., 5.));
    let ends = ends.borrow();
    assert_eq!(ends.len(), 1);
    assert_eq!(ends[0].global_position, Offset::new(55., 5.));
}

#[test]
fn mouse_drag_uses_the_precise_pan_slop() {
    // computePanSlop for a mouse is kPrecisePointerPanSlop (2).
    let starts = Rc::new(Cell::new(0));
    let mut detector = DragGestureDetector::with_kind(
        DragCallbacks {
            on_start: Some({
                let starts = starts.clone();
                Rc::new(move |_| starts.set(starts.get() + 1))
            }),
            ..DragCallbacks::default()
        },
        PointerDeviceKind::Mouse,
    );
    let now = Instant::now();
    detector.handle(drag_event(Offset::ZERO, PointerPhase::Down, now, 0));
    detector.handle(drag_event(Offset::new(2., 0.), PointerPhase::Move, now, 5));
    assert_eq!(starts.get(), 0, "the comparison is strict: 2 is not > 2");
    detector.handle(drag_event(
        Offset::new(2.5, 0.),
        PointerPhase::Move,
        now,
        10,
    ));
    assert_eq!(starts.get(), 1);
}

#[test]
fn pointer_recognizer_reports_drag_end_after_arena_updates() {
    // Flutter 3.41 monodrag.dart: the horizontal recognizer accepts once the
    // signed horizontal movement exceeds computeHitSlop (18), calls onStart at
    // that position, then onUpdate with primaryDelta for each later move, and
    // onEnd with the clamped fling velocity and the last pointer position.
    let ended = Rc::new(Cell::new(None));
    let mut recognizer = PointerGestureRecognizer::new(GestureCallbacks {
        on_horizontal_drag_update: Some(Rc::new(|_| {})),
        on_horizontal_drag_end: Some({
            let ended = ended.clone();
            Rc::new(move |details| ended.set(Some(details)))
        }),
        ..GestureCallbacks::default()
    });
    let now = Instant::now();
    assert_eq!(
        recognizer.observe(drag_event(Offset::ZERO, PointerPhase::Down, now, 0)),
        GestureDecision::Pending
    );
    let GestureAction::HorizontalDragStart(start) = accepted(
        &mut recognizer,
        drag_event(Offset::new(24., 0.), PointerPhase::Move, now, 10),
    ) else {
        panic!("the accepting move starts the drag");
    };
    assert_eq!(start.global_position, Offset::new(24., 0.));
    let GestureAction::HorizontalDrag(update) = accepted(
        &mut recognizer,
        drag_event(Offset::new(48., 4.), PointerPhase::Move, now, 20),
    ) else {
        panic!("later moves update the drag");
    };
    assert_eq!(update.delta, Offset::new(24., 0.), "only the drag axis");
    assert_eq!(update.primary_delta, Some(24.));
    let end = recognizer.observe(drag_event(Offset::new(48., 4.), PointerPhase::Up, now, 25));
    assert!(matches!(
        end,
        GestureDecision::Accept(GestureAction::HorizontalDragEnd(_))
    ));
    if let GestureDecision::Accept(action) = end {
        recognizer.dispatch(action);
    }
    let details = ended.get().expect("drag end callback");
    assert_eq!(details.global_position, Offset::new(48., 4.));
    let primary = details.primary_velocity.expect("an axis drag reports one");
    assert!(
        primary > 50. && primary <= 8000.,
        "fling velocity is above kMinFlingVelocity and clamped to kMaxFlingVelocity: {primary}"
    );
    assert_eq!(
        details.velocity.pixels_per_second,
        Offset::new(primary, 0.),
        "an axis fling drops the cross-axis component"
    );
}

#[test]
fn horizontal_drag_accepts_on_the_signed_axis_sum_not_the_path() {
    // `_globalDistanceMoved` accumulates `distance * primaryValue.sign`, so a
    // back-and-forth wobble never reaches the slop.
    let mut recognizer = PointerGestureRecognizer::new(GestureCallbacks {
        on_horizontal_drag_update: Some(Rc::new(|_| {})),
        ..GestureCallbacks::default()
    });
    let now = Instant::now();
    recognizer.observe(drag_event(Offset::ZERO, PointerPhase::Down, now, 0));
    for (x, millis) in [(10., 10), (0., 20), (10., 30), (0., 40)] {
        assert_eq!(
            recognizer.observe(drag_event(
                Offset::new(x, 0.),
                PointerPhase::Move,
                now,
                millis
            )),
            GestureDecision::Pending,
            "net movement stays below 18 at x={x}"
        );
    }
    recognizer.observe(drag_event(
        Offset::new(10., 0.),
        PointerPhase::Move,
        now,
        50,
    ));
    let action = accepted(
        &mut recognizer,
        drag_event(Offset::new(20., 0.), PointerPhase::Move, now, 60),
    );
    assert!(
        matches!(action, GestureAction::HorizontalDragStart(start) if start.global_position == Offset::new(20., 0.)),
        "{action:?}"
    );
}

#[test]
fn vertical_drag_wins_a_simultaneous_threshold_crossing() {
    // GestureDetector registers vertical, horizontal then pan recognizers; the
    // first one to accept wins the arena.
    let mut recognizer = PointerGestureRecognizer::new(GestureCallbacks {
        on_horizontal_drag_update: Some(Rc::new(|_| {})),
        on_vertical_drag_update: Some(Rc::new(|_| {})),
        on_pan_update: Some(Rc::new(|_| {})),
        ..GestureCallbacks::default()
    });
    let now = Instant::now();
    recognizer.observe(drag_event(Offset::ZERO, PointerPhase::Down, now, 0));
    let action = accepted(
        &mut recognizer,
        drag_event(Offset::new(20., 19.), PointerPhase::Move, now, 10),
    );
    assert!(
        matches!(action, GestureAction::VerticalDragStart(_)),
        "{action:?}"
    );
}

#[test]
fn cancelling_an_accepted_drag_ends_it() {
    // monodrag.dart didStopTrackingLastPointer: an accepted drag reports
    // onEnd, not onCancel, when its pointer is cancelled.
    let mut recognizer = PointerGestureRecognizer::new(GestureCallbacks {
        on_vertical_drag_update: Some(Rc::new(|_| {})),
        on_vertical_drag_end: Some(Rc::new(|_| {})),
        ..GestureCallbacks::default()
    });
    let now = Instant::now();
    recognizer.observe(drag_event(Offset::ZERO, PointerPhase::Down, now, 0));
    accepted(
        &mut recognizer,
        drag_event(Offset::new(0., 30.), PointerPhase::Move, now, 10),
    );
    assert!(matches!(
        recognizer.observe(drag_event(
            Offset::new(0., 30.),
            PointerPhase::Cancel,
            now,
            20
        )),
        GestureDecision::Accept(GestureAction::VerticalDragEnd(_))
    ));
}
