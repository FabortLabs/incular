//! Widget-level checks for Flutter 3.50.0's Shift + slash CharacterActivator
//! repeat tests. Incular starts with normalized KeyboardEvents; Flutter's
//! RawKeyEvent and ui.KeyData simulator transports are not equivalent here.

use incular_config::Constraints;
use incular_core::{Code, InputEvent, KeyboardEvent, KeyboardKey, Modifiers, NamedKey, Size};
use incular_runtime::Runtime;
use incular_widgets::{CharacterActivator, Focus, ShortcutRegistrar, ShortcutRegistry, SizedBox};
use std::{cell::Cell, rc::Rc};

fn shifted_question_mark_dispatch(activator: CharacterActivator, send_repeat: bool) -> [u32; 3] {
    let invoked = Rc::new(Cell::new(0));
    let count = Rc::clone(&invoked);
    let registry = ShortcutRegistry::new();
    let _registration = registry.add(activator, move || count.set(count.get() + 1));
    let target = Focus::new(SizedBox::shrink()).autofocus(true);
    let root = ShortcutRegistrar::with_registry(registry, target);
    let mut runtime = Runtime::new(root.into()).expect("mount shortcut scope");
    runtime
        .run_frame(Constraints::tight(Size::new(120., 40.)))
        .expect("lay out focused child");
    assert!(runtime.focused_element().is_some(), "child has focus");

    let shift = KeyboardKey::Named(NamedKey::Shift);
    let question_mark = KeyboardKey::Character("?".into());
    let mut send = |event, modifiers| {
        let _ = runtime.handle_input(InputEvent::Key(KeyboardEvent { modifiers, ..event }));
    };
    send(
        KeyboardEvent::key_down(shift.clone(), Code::ShiftLeft),
        Modifiers::SHIFT,
    );
    assert_eq!(invoked.get(), 0, "Shift alone is not the shortcut");

    let down = KeyboardEvent::key_down(question_mark.clone(), Code::Slash);
    send(down.clone(), Modifiers::SHIFT);
    let after_down = invoked.get();
    if send_repeat {
        send(
            KeyboardEvent {
                repeat: true,
                ..down
            },
            Modifiers::SHIFT,
        );
    }
    let after_repeat = invoked.get();
    send(
        KeyboardEvent::key_up(question_mark, Code::Slash),
        Modifiers::SHIFT,
    );
    send(
        KeyboardEvent::key_up(shift, Code::ShiftLeft),
        Modifiers::empty(),
    );
    [after_down, after_repeat, invoked.get()]
}

#[test]
fn registered_character_shortcut_handles_repeated_key_events_by_default() {
    assert_eq!(
        shifted_question_mark_dispatch(CharacterActivator::new("?"), true),
        [1, 2, 2]
    );
}

#[test]
fn registered_character_shortcut_can_reject_repeated_key_events() {
    assert_eq!(
        shifted_question_mark_dispatch(CharacterActivator::new("?").include_repeats(false), true),
        [1, 1, 1]
    );
}

#[test]
fn shift_slash_matches_the_produced_question_mark_once() {
    assert_eq!(
        shifted_question_mark_dispatch(CharacterActivator::new("?"), false),
        [1, 1, 1]
    );
}
