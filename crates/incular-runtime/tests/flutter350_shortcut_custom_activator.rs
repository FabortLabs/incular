use std::{cell::Cell, rc::Rc};

use incular_config::Constraints;
use incular_core::{Code, KeyboardEvent, KeyboardKey, Modifiers, NamedKey, Size};
use incular_runtime::Runtime;
use incular_widgets::{
    ActionScope, Actions, Focus, ShortcutActivator, ShortcutScope, Shortcuts, SingleActivator,
    SizedBox,
};

fn mount(bindings: Shortcuts<u8>, actions: Actions<u8>) -> Runtime {
    let child = Focus::new(SizedBox::shrink()).autofocus(true);
    let scope = ShortcutScope::new(Rc::new(bindings), child);
    let mut runtime = Runtime::new(ActionScope::new(Rc::new(actions), scope).into()).unwrap();
    runtime
        .run_frame(Constraints::tight(Size::new(20., 20.)))
        .unwrap();
    runtime
}

#[test]
fn an_unindexed_custom_activator_is_consulted_after_the_keyed_candidate() {
    let c = KeyboardKey::Character("c".into());
    let fallback_key = c.clone();
    let fallback = ShortcutActivator::custom(None, move |event, _| {
        event.state.is_down() && event.key == fallback_key
    });
    let mut bindings = Shortcuts::<u8>::typed();
    bindings.bind_activator(fallback, 1);
    bindings.bind_activator(SingleActivator::new(c.clone(), Modifiers::CONTROL), 10);
    let count = Rc::new(Cell::new(0));
    let mut actions = Actions::<u8>::typed();
    for value in [1, 10] {
        let count = count.clone();
        actions.register(value, move || count.set(count.get() + value));
    }
    let mut runtime = mount(bindings, actions);
    let control = KeyboardKey::Named(NamedKey::Control);
    let shift = KeyboardKey::Named(NamedKey::Shift);
    let mut send = |key: &KeyboardKey, code, down, modifiers| {
        let mut event = if down {
            KeyboardEvent::key_down(key.clone(), code)
        } else {
            KeyboardEvent::key_up(key.clone(), code)
        };
        event.modifiers = modifiers;
        runtime.dispatch_keyboard_event(event)
    };

    let empty = Modifiers::empty();
    let ctrl = Modifiers::CONTROL;
    let shifted = Modifiers::SHIFT;
    for (control_down, shift_down, hits) in [(false, false, 1), (true, false, 10), (true, true, 1)]
    {
        count.set(0);
        let mut mods = empty;
        if shift_down {
            mods |= shifted;
            assert!(!send(&shift, Code::ShiftLeft, true, mods));
        }
        if control_down {
            mods |= ctrl;
            assert!(!send(&control, Code::ControlLeft, true, mods));
        }
        assert_eq!(count.get(), 0);
        assert!(send(&c, Code::KeyC, true, mods));
        assert_eq!(count.get(), hits);
        assert!(!send(&c, Code::KeyC, false, mods));
        assert_eq!(count.get(), hits);
        if control_down {
            mods.remove(ctrl);
            assert!(!send(&control, Code::ControlLeft, false, mods));
        }
        if shift_down {
            mods.remove(shifted);
            assert!(!send(&shift, Code::ShiftLeft, false, mods));
        }
        assert_eq!(count.get(), hits);
    }
    assert!(
        runtime
            .hardware_keyboard()
            .logical_keys_pressed()
            .is_empty()
    );
}

#[test]
fn indexed_custom_activators_do_not_run_for_unadvertised_keys() {
    let a = KeyboardKey::Character("a".into());
    let c = KeyboardKey::Character("c".into());
    let predicate_calls = Rc::new(Cell::new(0));
    let invoked = Rc::new(Cell::new(0));
    let checked = predicate_calls.clone();
    let custom = ShortcutActivator::custom(Some(vec![c.clone()]), move |event, state| {
        checked.set(checked.get() + 1);
        event.state.is_down() && state.is_some_and(|state| state.is_logical_key_pressed(&event.key))
    });
    let mut bindings = Shortcuts::<u8>::typed();
    bindings.bind_activator(custom.clone(), 1);
    assert!(bindings.clear_activator(&custom));
    assert!(bindings.is_empty());
    bindings.bind_activator(custom.clone(), 1);
    bindings.bind_activator(custom, 2);

    let mut actions = Actions::<u8>::typed();
    actions.register(1u8, || {
        panic!("cloning an activator must not duplicate a binding")
    });
    let count = invoked.clone();
    actions.register(2u8, move || count.set(count.get() + 1));
    let mut runtime = mount(bindings, actions);
    assert!(!runtime.dispatch_keyboard_event(KeyboardEvent::key_down(a.clone(), Code::KeyA)));
    assert!(!runtime.dispatch_keyboard_event(KeyboardEvent::key_up(a, Code::KeyA)));
    assert_eq!(predicate_calls.get(), 0);
    assert!(runtime.dispatch_keyboard_event(KeyboardEvent::key_down(c.clone(), Code::KeyC)));
    assert!(!runtime.dispatch_keyboard_event(KeyboardEvent::key_up(c, Code::KeyC)));
    assert_eq!(predicate_calls.get(), 2);
    assert_eq!(invoked.get(), 1);
}
