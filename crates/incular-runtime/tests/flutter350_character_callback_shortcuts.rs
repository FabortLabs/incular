use std::{cell::Cell, rc::Rc};

use incular_config::Constraints;
use incular_core::{Code, KeyboardEvent, KeyboardKey, Modifiers, NamedKey, Size};
use incular_runtime::Runtime;
use incular_widgets::{Actions, Command, LogicalShortcutKey, Shortcuts};
use incular_widgets::{
    CallbackShortcuts, CharacterActivator, Focus, KeyboardListener, SizedBox, Widget,
};

fn mount(child: impl Into<Widget>) -> Runtime {
    let mut runtime = Runtime::new(child.into()).unwrap();
    runtime
        .run_frame(Constraints::tight(Size::new(20.0, 20.0)))
        .unwrap();
    assert!(runtime.focused_element().is_some());
    runtime
}

fn child() -> Focus {
    Focus::new(SizedBox::shrink()).autofocus(true)
}

fn increment(count: &Rc<Cell<usize>>) -> impl Fn() + 'static {
    let count = count.clone();
    move || count.set(count.get() + 1)
}

fn character(c: &str) -> KeyboardKey {
    KeyboardKey::Character(c.to_owned())
}

fn send(
    runtime: &mut Runtime,
    key: KeyboardKey,
    code: Code,
    down: bool,
    modifiers: Modifiers,
) -> bool {
    let mut event = if down {
        KeyboardEvent::key_down(key, code)
    } else {
        KeyboardEvent::key_up(key, code)
    };
    event.modifiers = modifiers;
    runtime.dispatch_keyboard_event(event)
}

fn letter(runtime: &mut Runtime, c: &str, code: Code, down: bool) -> bool {
    send(runtime, character(c), code, down, Modifiers::empty())
}

#[test]
fn nonoverlapping_character_callbacks_work_across_nested_scopes() {
    let outer = Rc::new(Cell::new(0));
    let inner = Rc::new(Cell::new(0));
    let nested =
        CallbackShortcuts::new(child()).shortcut(CharacterActivator::new("a"), increment(&inner));
    let mut runtime = mount(
        CallbackShortcuts::new(nested).shortcut(CharacterActivator::new("b"), increment(&outer)),
    );

    assert!(letter(&mut runtime, "a", Code::KeyA, true));
    assert_eq!((outer.get(), inner.get()), (0, 1));
    assert!(letter(&mut runtime, "b", Code::KeyB, true));
    assert_eq!((outer.get(), inner.get()), (1, 1));
    assert!(!letter(&mut runtime, "a", Code::KeyA, false));
    assert!(!letter(&mut runtime, "b", Code::KeyB, false));
    assert_eq!((outer.get(), inner.get()), (1, 1));
}

#[test]
fn character_callbacks_and_intent_shortcuts_follow_focus_ancestry() {
    let inner_a = Rc::new(Cell::new(0));
    let outer_b = Rc::new(Cell::new(0));
    let action_a = Rc::new(Cell::new(0));
    let action_b = Rc::new(Cell::new(0));
    let a = Command::new("a");
    let b = Command::new("b");
    let mut actions = Actions::new();
    actions.register(a.clone(), increment(&action_a));
    actions.register(b.clone(), increment(&action_b));
    let mut shortcuts = Shortcuts::new();
    shortcuts.bind_logical(
        LogicalShortcutKey::new(character("a"), Modifiers::empty()),
        a,
    );
    shortcuts.bind_logical(
        LogicalShortcutKey::new(character("b"), Modifiers::empty()),
        b,
    );
    let inner =
        CallbackShortcuts::new(child()).shortcut(CharacterActivator::new("a"), increment(&inner_a));
    let command_scope =
        KeyboardListener::new(inner).with_shortcuts(Rc::new(shortcuts), Rc::new(actions));
    let mut runtime = mount(
        CallbackShortcuts::new(command_scope)
            .shortcut(CharacterActivator::new("b"), increment(&outer_b)),
    );

    let counts = || (inner_a.get(), outer_b.get(), action_a.get(), action_b.get());
    assert!(letter(&mut runtime, "a", Code::KeyA, true));
    assert_eq!(counts(), (1, 0, 0, 0));
    assert!(!letter(&mut runtime, "a", Code::KeyA, false));
    assert_eq!(counts(), (1, 0, 0, 0));
    inner_a.set(0);
    assert!(letter(&mut runtime, "b", Code::KeyB, true));
    assert_eq!(counts(), (0, 0, 0, 1));
    assert!(!letter(&mut runtime, "b", Code::KeyB, false));
    assert_eq!(counts(), (0, 0, 0, 1));
}

#[test]
fn produced_question_mark_requires_alt_ctrl_and_meta_but_ignores_shift_as_modifier() {
    let hits = Rc::new(Cell::new(0));
    let activator = CharacterActivator::new("?")
        .alt(true)
        .control(true)
        .meta(true);
    let mut runtime = mount(CallbackShortcuts::new(child()).shortcut(activator, increment(&hits)));
    let tap_slash = |runtime: &mut Runtime, modifiers, expected| {
        assert_eq!(
            send(runtime, character("?"), Code::Slash, true, modifiers),
            expected
        );
        assert!(!send(
            runtime,
            character("?"),
            Code::Slash,
            false,
            modifiers
        ));
    };
    let mut modifiers = Modifiers::SHIFT;
    assert!(!send(
        &mut runtime,
        KeyboardKey::Named(NamedKey::Shift),
        Code::ShiftLeft,
        true,
        modifiers
    ));
    tap_slash(&mut runtime, modifiers, false);
    assert_eq!(hits.get(), 0);

    for (side, keys) in [
        (
            Code::ShiftLeft,
            [Code::AltLeft, Code::MetaLeft, Code::ControlLeft],
        ),
        (
            Code::ShiftRight,
            [Code::AltRight, Code::MetaRight, Code::ControlRight],
        ),
    ] {
        let set = [
            (NamedKey::Alt, Modifiers::ALT),
            (NamedKey::Meta, Modifiers::META),
            (NamedKey::Control, Modifiers::CONTROL),
        ];
        if side == Code::ShiftRight {
            modifiers = Modifiers::SHIFT;
            assert!(!send(
                &mut runtime,
                KeyboardKey::Named(NamedKey::Shift),
                side,
                true,
                modifiers
            ));
        }
        for ((key, bit), code) in set.into_iter().zip(keys) {
            modifiers.insert(bit);
            assert!(!send(
                &mut runtime,
                KeyboardKey::Named(key),
                code,
                true,
                modifiers
            ));
        }
        assert_eq!(hits.get(), 0);
        tap_slash(&mut runtime, modifiers, true);
        assert_eq!(hits.get(), 1);
        modifiers.remove(Modifiers::SHIFT);
        assert!(!send(
            &mut runtime,
            KeyboardKey::Named(NamedKey::Shift),
            side,
            false,
            modifiers
        ));
        for i in [1, 0, 2] {
            let (key, bit) = set[i];
            modifiers.remove(bit);
            assert!(!send(
                &mut runtime,
                KeyboardKey::Named(key),
                keys[i],
                false,
                modifiers
            ));
        }
        assert_eq!(hits.get(), 1);
        hits.set(0);
    }
    assert!(
        runtime
            .hardware_keyboard()
            .logical_keys_pressed()
            .is_empty()
    );
}
