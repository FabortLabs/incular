use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use incular_config::Constraints;
use incular_core::{Code, KeyboardEvent, KeyboardKey, Modifiers, NamedKey, Size};
use incular_runtime::Runtime;
use incular_widgets::{
    Builder, CharacterActivator, Focus, LockState, ShortcutRegistrar, ShortcutRegistry,
    SingleActivator, SizedBox, Widget, WidgetsApp,
};

fn mount(child: Widget) -> Runtime {
    let mut runtime = Runtime::new(child).unwrap();
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

fn send(runtime: &mut Runtime, key: KeyboardKey, code: Code, down: bool) -> bool {
    runtime.dispatch_keyboard_event(if down {
        KeyboardEvent::key_down(key, code)
    } else {
        KeyboardEvent::key_up(key, code)
    })
}

fn letter(runtime: &mut Runtime, letter: &str, code: Code, down: bool) -> bool {
    send(
        runtime,
        KeyboardKey::Character(letter.to_owned()),
        code,
        down,
    )
}

fn activator(letter: &str) -> SingleActivator {
    SingleActivator::new(
        KeyboardKey::Character(letter.to_owned()),
        Modifiers::empty(),
    )
}

#[test]
fn registered_shortcuts_fire_for_their_own_key_down_only() {
    let registry = ShortcutRegistry::new();
    let a = Rc::new(Cell::new(0));
    let b = Rc::new(Cell::new(0));
    let _a = registry.add(activator("a"), increment(&a));
    let _b = registry.add(activator("b"), increment(&b));
    let mut runtime = mount(ShortcutRegistrar::with_registry(registry, child()).into());

    assert!(letter(&mut runtime, "a", Code::KeyA, true));
    assert_eq!((a.get(), b.get()), (1, 0));
    assert!(!letter(&mut runtime, "a", Code::KeyA, false));
    assert_eq!((a.get(), b.get()), (1, 0));
    a.set(0);
    assert!(letter(&mut runtime, "b", Code::KeyB, true));
    assert_eq!((a.get(), b.get()), (0, 1));
    assert!(!letter(&mut runtime, "b", Code::KeyB, false));
    assert_eq!((a.get(), b.get()), (0, 1));
}

#[test]
fn widgets_app_provides_a_live_registrar_to_its_home_route() {
    let a = Rc::new(Cell::new(0));
    let b = Rc::new(Cell::new(0));
    let registrations = Rc::new(RefCell::new(None));
    let home = Builder::new({
        let (a, b, registrations) = (a.clone(), b.clone(), registrations.clone());
        move |context| {
            let registry = ShortcutRegistry::maybe_of(context).expect("WidgetsApp owns a registry");
            let mut registrations = registrations.borrow_mut();
            if registrations.is_none() {
                *registrations = Some([
                    registry.add(activator("a"), increment(&a)),
                    registry.add(activator("b"), increment(&b)),
                ]);
            }
            child().into()
        }
    });
    let mut runtime = mount(WidgetsApp::new(home).into());
    assert!(registrations.borrow().is_some());

    assert!(letter(&mut runtime, "a", Code::KeyA, true));
    assert_eq!((a.get(), b.get()), (1, 0));
    assert!(!letter(&mut runtime, "a", Code::KeyA, false));
    assert_eq!((a.get(), b.get()), (1, 0));
    a.set(0);
    assert!(letter(&mut runtime, "b", Code::KeyB, true));
    assert_eq!((a.get(), b.get()), (0, 1));
    assert!(!letter(&mut runtime, "b", Code::KeyB, false));
    assert_eq!((a.get(), b.get()), (0, 1));
}

#[test]
fn nested_registrar_consumes_matching_keys_before_outer_registrar() {
    let outer = ShortcutRegistry::new();
    let inner = ShortcutRegistry::new();
    let outer_hits = Rc::new(Cell::new(0));
    let inner_hits = Rc::new(Cell::new(0));
    let _outer = outer.add(activator("a"), increment(&outer_hits));
    let _inner = inner.add(activator("a"), increment(&inner_hits));
    let nested = ShortcutRegistrar::with_registry(inner, child());
    let mut runtime = mount(ShortcutRegistrar::with_registry(outer, nested).into());

    assert!(letter(&mut runtime, "a", Code::KeyA, true));
    assert_eq!((outer_hits.get(), inner_hits.get()), (0, 1));
    assert!(!letter(&mut runtime, "a", Code::KeyA, false));
    assert_eq!((outer_hits.get(), inner_hits.get()), (0, 1));
}

#[test]
fn nonoverlapping_nested_registrars_route_each_key_to_its_binding() {
    let outer = ShortcutRegistry::new();
    let inner = ShortcutRegistry::new();
    let outer_hits = Rc::new(Cell::new(0));
    let inner_hits = Rc::new(Cell::new(0));
    let _outer = outer.add(CharacterActivator::new("b"), increment(&outer_hits));
    let _inner = inner.add(CharacterActivator::new("a"), increment(&inner_hits));
    let nested = ShortcutRegistrar::with_registry(inner, child());
    let mut runtime = mount(ShortcutRegistrar::with_registry(outer, nested).into());

    assert!(letter(&mut runtime, "a", Code::KeyA, true));
    assert_eq!((outer_hits.get(), inner_hits.get()), (0, 1));
    assert!(letter(&mut runtime, "b", Code::KeyB, true));
    assert_eq!((outer_hits.get(), inner_hits.get()), (1, 1));
    assert!(!letter(&mut runtime, "a", Code::KeyA, false));
    assert!(!letter(&mut runtime, "b", Code::KeyB, false));
    assert_eq!((outer_hits.get(), inner_hits.get()), (1, 1));
}

#[test]
fn registrar_uses_hardware_num_lock_when_event_modifiers_do_not_include_it() {
    let registry = ShortcutRegistry::new();
    let hits = Rc::new(Cell::new(0));
    let _binding = registry.add(
        activator("4").with_num_lock(LockState::Locked),
        increment(&hits),
    );
    let mut runtime = mount(ShortcutRegistrar::with_registry(registry.clone(), child()).into());

    let lock = KeyboardKey::Named(NamedKey::NumLock);
    for (step, expected) in [false, true, false].into_iter().enumerate() {
        assert_eq!(
            runtime
                .hardware_keyboard()
                .lock_modes_enabled()
                .contains(Modifiers::NUM_LOCK),
            expected
        );
        assert_eq!(letter(&mut runtime, "4", Code::Numpad4, true), expected);
        assert!(!letter(&mut runtime, "4", Code::Numpad4, false));
        if step < 2 {
            assert!(!send(&mut runtime, lock.clone(), Code::NumLock, true));
            assert!(!send(&mut runtime, lock.clone(), Code::NumLock, false));
        }
    }
    assert_eq!(hits.get(), 1);
    assert_eq!(registry.modifiers(), Modifiers::empty());
    assert!(
        runtime
            .hardware_keyboard()
            .logical_keys_pressed()
            .is_empty()
    );
}
