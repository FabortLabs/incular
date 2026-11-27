use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use incular_config::Constraints;
use incular_core::{Code, KeyboardEvent, KeyboardKey, Modifiers, NamedKey, Size};
use incular_runtime::Runtime;
use incular_widgets::{
    ActionScope, Actions, CallbackShortcuts, Focus, Intent, KeyEventResult, LogicalKeySet,
    ShortcutScope, Shortcuts, SizedBox, Widget,
};

type Key = (Code, &'static str, Option<NamedKey>);
const A: Key = (Code::KeyA, "a", None);
const C: Key = (Code::KeyC, "c", None);
const LEFT_CONTROL: Key = (Code::ControlLeft, "", Some(NamedKey::Control));
const RIGHT_CONTROL: Key = (Code::ControlRight, "", Some(NamedKey::Control));
const SHIFT: Key = (Code::ShiftLeft, "", Some(NamedKey::Shift));

fn logical((_, character, named): Key) -> KeyboardKey {
    named.map_or_else(
        || KeyboardKey::Character(character.to_owned()),
        KeyboardKey::Named,
    )
}

fn send(runtime: &mut Runtime, key: Key, down: bool, modifiers: Modifiers) {
    let mut event = if down {
        KeyboardEvent::key_down(logical(key), key.0)
    } else {
        KeyboardEvent::key_up(logical(key), key.0)
    };
    event.modifiers = modifiers;
    let _ = runtime.dispatch_keyboard_event(event);
}

fn mount(root: impl Into<Widget>) -> Runtime {
    let mut runtime = Runtime::new(root.into()).unwrap();
    runtime
        .run_frame(Constraints::tight(Size::new(100., 100.)))
        .unwrap();
    assert!(runtime.focused_element().is_some());
    runtime
}

fn child() -> Focus {
    Focus::new(SizedBox::new().width(20.).height(20.)).autofocus(true)
}

#[test]
fn control_and_c_activate_in_either_order_and_no_other_key_can_be_held() {
    let calls = Rc::new(Cell::new(0));
    let counter = calls.clone();
    let mut actions = Actions::<()>::typed();
    actions.register((), move || counter.set(counter.get() + 1));
    let mut shortcuts = Shortcuts::<()>::typed();
    shortcuts.bind_key_set(
        LogicalKeySet::new([logical(C), logical(LEFT_CONTROL)]),
        Intent::new(()),
    );
    let runtime = mount(ActionScope::new(
        Rc::new(actions),
        ShortcutScope::new(Rc::new(shortcuts), child()),
    ));
    verify_control_c_sequences(runtime, calls);
}

#[test]
fn callback_shortcuts_use_the_same_exact_held_state() {
    let calls = Rc::new(Cell::new(0));
    let counter = calls.clone();
    let runtime = mount(CallbackShortcuts::new(child()).shortcut(
        LogicalKeySet::new([logical(C), logical(LEFT_CONTROL)]),
        move || counter.set(counter.get() + 1),
    ));
    verify_control_c_sequences(runtime, calls);
}

fn verify_control_c_sequences(mut runtime: Runtime, calls: Rc<Cell<i32>>) {
    let ctrl = Modifiers::CONTROL;
    let empty = Modifiers::empty();
    let ctrl_shift = ctrl | Modifiers::SHIFT;
    for (presses, releases, count) in [
        (
            vec![(LEFT_CONTROL, ctrl), (C, ctrl)],
            vec![(C, ctrl), (LEFT_CONTROL, empty)],
            1,
        ),
        (
            vec![(C, empty), (LEFT_CONTROL, ctrl)],
            vec![(C, ctrl), (LEFT_CONTROL, empty)],
            1,
        ),
        (
            vec![(RIGHT_CONTROL, ctrl), (C, ctrl)],
            vec![(C, ctrl), (RIGHT_CONTROL, empty)],
            1,
        ),
        (
            vec![(LEFT_CONTROL, ctrl), (SHIFT, ctrl_shift), (C, ctrl_shift)],
            vec![
                (C, ctrl_shift),
                (LEFT_CONTROL, Modifiers::SHIFT),
                (SHIFT, empty),
            ],
            0,
        ),
        (
            vec![(LEFT_CONTROL, ctrl), (A, ctrl), (C, ctrl)],
            vec![(C, ctrl), (LEFT_CONTROL, empty), (A, empty)],
            0,
        ),
    ] {
        calls.set(0);
        let last = presses.len() - 1;
        for (index, (key, modifiers)) in presses.into_iter().enumerate() {
            send(&mut runtime, key, true, modifiers);
            assert_eq!(
                calls.get(),
                if index == last { count } else { 0 },
                "down: {:?}",
                key.0
            );
        }
        for (key, modifiers) in releases {
            send(&mut runtime, key, false, modifiers);
            assert_eq!(calls.get(), count, "up: {:?}", key.0);
        }
        assert!(
            runtime
                .hardware_keyboard()
                .logical_keys_pressed()
                .is_empty()
        );
    }
}

#[test]
fn activation_checks_actual_focused_events_including_repeat_and_release() {
    let received = Rc::new(RefCell::new(Vec::new()));
    let events = received.clone();
    let mut runtime = mount(child().on_key_event(move |_, event| {
        events.borrow_mut().push(event);
        KeyEventResult::Ignored
    }));
    let set = LogicalKeySet::new([logical(A), logical(LEFT_CONTROL)]);
    let ctrl = Modifiers::CONTROL;
    send(&mut runtime, LEFT_CONTROL, true, ctrl);
    for (event, expected) in [
        (
            KeyboardEvent {
                modifiers: ctrl,
                ..KeyboardEvent::key_down(logical(A), A.0)
            },
            true,
        ),
        (
            KeyboardEvent {
                modifiers: ctrl,
                repeat: true,
                ..KeyboardEvent::key_down(logical(A), A.0)
            },
            true,
        ),
        (
            KeyboardEvent {
                modifiers: ctrl,
                ..KeyboardEvent::key_up(logical(A), A.0)
            },
            false,
        ),
        (
            KeyboardEvent::key_up(logical(LEFT_CONTROL), LEFT_CONTROL.0),
            false,
        ),
    ] {
        let before = received.borrow().len();
        let _ = runtime.dispatch_keyboard_event(event);
        let received = received.borrow();
        assert_eq!(received.len(), before + 1);
        assert_eq!(
            set.accepts_with_keyboard_state(received.last().unwrap(), runtime.hardware_keyboard()),
            expected
        );
    }
}

#[test]
fn distinct_nonmodifier_members_and_duplicate_physical_control_keys_can_form_chords() {
    let calls = Rc::new(Cell::new(0));
    let counter = calls.clone();
    let mut runtime = mount(CallbackShortcuts::new(child()).shortcut(
        LogicalKeySet::new([logical(C), logical(A), logical(LEFT_CONTROL)]),
        move || counter.set(counter.get() + 1),
    ));
    let ctrl = Modifiers::CONTROL;

    send(&mut runtime, LEFT_CONTROL, true, ctrl);
    send(&mut runtime, RIGHT_CONTROL, true, ctrl);
    send(&mut runtime, C, true, ctrl);
    assert_eq!(calls.get(), 0);
    send(&mut runtime, A, true, ctrl);
    assert_eq!(calls.get(), 1);
    send(&mut runtime, LEFT_CONTROL, false, ctrl);
    send(&mut runtime, A, false, ctrl);
    assert_eq!(calls.get(), 1);
    send(&mut runtime, A, true, ctrl);
    assert_eq!(calls.get(), 2);
    send(&mut runtime, RIGHT_CONTROL, false, Modifiers::empty());
    send(&mut runtime, C, false, Modifiers::empty());
    send(&mut runtime, C, true, Modifiers::empty());
    assert_eq!(calls.get(), 2);
}

#[test]
fn ordinary_chords_accept_either_order_and_repeat_only_while_the_set_is_exact() {
    let chords = Rc::new(Cell::new(0));
    let singles = Rc::new(Cell::new(0));
    let chord_calls = chords.clone();
    let single_calls = singles.clone();
    let b: Key = (Code::KeyB, "b", None);
    let none = Modifiers::empty();
    let mut runtime = mount(
        CallbackShortcuts::new(child())
            .shortcut(LogicalKeySet::new([logical(A), logical(b)]), move || {
                chord_calls.set(chord_calls.get() + 1);
            })
            .shortcut(
                incular_widgets::SingleActivator::new(logical(b), none),
                move || {
                    single_calls.set(single_calls.get() + 1);
                },
            ),
    );
    let counts = || (chords.get(), singles.get());
    let repeat_b = |runtime: &mut Runtime| {
        let _ = runtime.dispatch_keyboard_event(KeyboardEvent {
            repeat: true,
            ..KeyboardEvent::key_down(logical(b), b.0)
        });
    };

    send(&mut runtime, A, true, none);
    assert_eq!(counts(), (0, 0));
    send(&mut runtime, b, true, none);
    assert_eq!(counts(), (1, 1));
    repeat_b(&mut runtime);
    assert_eq!(counts(), (2, 2));
    send(&mut runtime, b, false, none);
    send(&mut runtime, A, false, none);
    assert_eq!(counts(), (2, 2));

    send(&mut runtime, b, true, none);
    assert_eq!(counts(), (2, 3));
    send(&mut runtime, A, true, none);
    assert_eq!(counts(), (3, 3));
    send(&mut runtime, b, false, none);
    send(&mut runtime, A, false, none);
    assert_eq!(counts(), (3, 3));

    send(&mut runtime, C, true, none);
    send(&mut runtime, A, true, none);
    send(&mut runtime, b, true, none);
    assert_eq!(counts(), (3, 4));
    repeat_b(&mut runtime);
    assert_eq!(counts(), (3, 5));
    send(&mut runtime, C, false, none);
    assert_eq!(counts(), (3, 5));
    repeat_b(&mut runtime);
    assert_eq!(counts(), (4, 6));
    send(&mut runtime, b, false, none);
    send(&mut runtime, A, false, none);
    assert_eq!(counts(), (4, 6));
    assert!(
        runtime
            .hardware_keyboard()
            .logical_keys_pressed()
            .is_empty()
    );
}
