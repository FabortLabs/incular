use std::{cell::Cell, rc::Rc};

use incular_config::Constraints;
use incular_core::{Code, KeyboardEvent, KeyboardKey, Modifiers, NamedKey, Size};
use incular_runtime::Runtime;
use incular_widgets::{
    CallbackShortcuts, FocusNode, KeyboardListener, LockState, ShortcutKey, SingleActivator,
    SizedBox,
};

fn num_lock(runtime: &mut Runtime) {
    let logical = KeyboardKey::Named(NamedKey::NumLock);
    assert!(
        !runtime.dispatch_keyboard_event(KeyboardEvent::key_down(logical.clone(), Code::NumLock))
    );
    assert!(!runtime.dispatch_keyboard_event(KeyboardEvent::key_up(logical, Code::NumLock)));
}

fn case(policy: LockState, locked_matches: bool, unlocked_matches: bool) {
    let logical = KeyboardKey::Character("4".to_owned());
    let activator = SingleActivator::new(logical.clone(), Modifiers::empty()).with_num_lock(policy);
    let node = FocusNode::new();
    let hits = Rc::new(Cell::new(0));
    let child = KeyboardListener::new(SizedBox::shrink()).focus_node(node.clone());
    let shortcuts = CallbackShortcuts::new(child).shortcut(activator.clone(), {
        let hits = hits.clone();
        move || hits.set(hits.get() + 1)
    });
    let mut runtime = Runtime::new(shortcuts.into()).expect("runtime");
    let bounds = Constraints::tight(Size::new(20.0, 20.0));
    runtime.run_frame(bounds).unwrap();
    node.request_focus();
    runtime.run_frame(bounds).unwrap();
    assert!(node.has_primary_focus());
    for (locked, expected) in [(true, locked_matches), (false, unlocked_matches)] {
        num_lock(&mut runtime);
        let locks = runtime.hardware_keyboard().lock_modes_enabled();
        assert_eq!(locks.contains(Modifiers::NUM_LOCK), locked);
        let down = KeyboardEvent::key_down(logical.clone(), Code::Numpad4);
        assert_eq!(activator.accepts_with_lock_modes(&down, locks), expected);
        let before = hits.get();
        assert_eq!(runtime.dispatch_keyboard_event(down), expected);
        assert_eq!(hits.get(), before + usize::from(expected));
        assert!(
            !runtime.dispatch_keyboard_event(KeyboardEvent::key_up(logical.clone(), Code::Numpad4))
        );
    }
}

#[test]
fn locked_matches_only_when_num_lock_is_on() {
    case(LockState::Locked, true, false);
}

#[test]
fn unlocked_matches_only_when_num_lock_is_off() {
    case(LockState::Unlocked, false, true);
}

#[test]
fn ignored_matches_in_both_states() {
    case(LockState::Ignored, true, true);
}

#[test]
fn observed_lock_does_not_rewrite_the_native_modifiers_of_physical_shortcuts() {
    let node = FocusNode::new();
    let child = KeyboardListener::new(SizedBox::shrink()).focus_node(node.clone());
    let physical = ShortcutKey::new(Code::Numpad4, Modifiers::empty());
    let mut runtime = Runtime::new(
        CallbackShortcuts::new(child)
            .shortcut(physical, || {})
            .into(),
    )
    .unwrap();
    let bounds = Constraints::tight(Size::new(20.0, 20.0));
    runtime.run_frame(bounds).unwrap();
    node.request_focus();
    runtime.run_frame(bounds).unwrap();
    num_lock(&mut runtime);
    assert!(runtime.dispatch_keyboard_event(KeyboardEvent::key_down(
        KeyboardKey::Character("4".to_owned()),
        Code::Numpad4
    )));
}

fn named(letter: &str) -> SingleActivator {
    SingleActivator::new(
        KeyboardKey::Character(letter.to_owned()),
        Modifiers::empty(),
    )
}

fn mounted(child: CallbackShortcuts, node: &FocusNode) -> Runtime {
    let mut runtime = Runtime::new(child.into()).unwrap();
    let bounds = Constraints::tight(Size::new(20.0, 20.0));
    runtime.run_frame(bounds).unwrap();
    node.request_focus();
    runtime.run_frame(bounds).unwrap();
    assert!(node.has_primary_focus());
    runtime
}

fn event(letter: &str, code: Code, down: bool) -> KeyboardEvent {
    let key = KeyboardKey::Character(letter.to_owned());
    if down {
        KeyboardEvent::key_down(key, code)
    } else {
        KeyboardEvent::key_up(key, code)
    }
}

fn increment(counter: &Rc<Cell<usize>>) -> impl Fn() + 'static {
    let counter = counter.clone();
    move || counter.set(counter.get() + 1)
}

#[test]
fn distinct_callback_bindings_trigger_on_their_own_down_events() {
    let node = FocusNode::new();
    let a = Rc::new(Cell::new(0));
    let b = Rc::new(Cell::new(0));
    let child = KeyboardListener::new(SizedBox::shrink()).focus_node(node.clone());
    let mut runtime = mounted(
        CallbackShortcuts::new(child)
            .shortcut(named("a"), increment(&a))
            .shortcut(named("b"), increment(&b)),
        &node,
    );
    assert!(runtime.dispatch_keyboard_event(event("a", Code::KeyA, true)));
    assert_eq!((a.get(), b.get()), (1, 0));
    assert!(!runtime.dispatch_keyboard_event(event("a", Code::KeyA, false)));
    assert_eq!((a.get(), b.get()), (1, 0));
    a.set(0);
    b.set(0);
    assert!(runtime.dispatch_keyboard_event(event("b", Code::KeyB, true)));
    assert_eq!((a.get(), b.get()), (0, 1));
    assert!(!runtime.dispatch_keyboard_event(event("b", Code::KeyB, false)));
    assert_eq!((a.get(), b.get()), (0, 1));
}

#[test]
fn matching_inner_callback_stops_the_outer_scope() {
    let node = FocusNode::new();
    let inner = Rc::new(Cell::new(0));
    let outer = Rc::new(Cell::new(0));
    let child = KeyboardListener::new(SizedBox::shrink()).focus_node(node.clone());
    let nested = CallbackShortcuts::new(child).shortcut(named("a"), increment(&inner));
    let mut runtime = mounted(
        CallbackShortcuts::new(nested).shortcut(named("a"), increment(&outer)),
        &node,
    );
    assert!(runtime.dispatch_keyboard_event(event("a", Code::KeyA, true)));
    assert_eq!((outer.get(), inner.get()), (0, 1));
    assert!(!runtime.dispatch_keyboard_event(event("a", Code::KeyA, false)));
    assert_eq!((outer.get(), inner.get()), (0, 1));
}

#[test]
fn distinct_matching_activators_all_run_and_equal_ones_replace_in_place() {
    let node = FocusNode::new();
    let order = Rc::new(std::cell::RefCell::new(Vec::new()));
    let record = |value| {
        let order = order.clone();
        move || order.borrow_mut().push(value)
    };
    let child = KeyboardListener::new(SizedBox::shrink()).focus_node(node.clone());
    let mut runtime = mounted(
        CallbackShortcuts::new(child)
            .shortcut(named("a"), record(1))
            .shortcut(named("a").with_repeats(false), record(2))
            .shortcut(named("a"), record(3)),
        &node,
    );
    assert!(runtime.dispatch_keyboard_event(event("a", Code::KeyA, true)));
    assert_eq!(order.take(), [3, 2]);
}
