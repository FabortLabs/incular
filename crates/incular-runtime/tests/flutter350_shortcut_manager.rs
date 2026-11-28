//! External shortcut managers retain identity and route mounted keys.
use std::{cell::RefCell, rc::Rc};

use incular_config::Constraints;
use incular_core::{Code, KeyState, KeyboardEvent, KeyboardKey, Modifiers, NamedKey, Size};
use incular_runtime::Runtime;
use incular_widgets::{
    ActionScope, Actions, Focus, FocusNode, KeyEventResult, LogicalKeySet, ShortcutManager,
    ShortcutScope, Shortcuts, SizedBox, Widget,
};

type Calls = Rc<RefCell<Vec<&'static str>>>;

fn bounds() -> Constraints {
    Constraints::tight(Size::new(160., 80.))
}

fn mount(widget: impl Into<Widget>) -> Runtime {
    let mut runtime = Runtime::new(widget.into()).unwrap();
    runtime.run_frame(bounds()).unwrap();
    runtime
}

fn map(key: KeyboardKey) -> Shortcuts<u8> {
    let mut map = Shortcuts::typed();
    map.bind_key_set(LogicalKeySet::new([key]), 1);
    map
}

fn actions(child: impl Into<Widget>, calls: Calls) -> ActionScope {
    let mut actions = Actions::<u8>::typed();
    actions.register(1, move || calls.borrow_mut().push("action"));
    ActionScope::new(Rc::new(actions), child)
}

fn observe(manager: &ShortcutManager<u8>, calls: Calls) {
    manager.set_key_observer(Some(Rc::new(move |event| {
        if event.state == KeyState::Down {
            calls.borrow_mut().push(match event.code {
                Code::ShiftLeft => "left Shift",
                Code::KeyA => "A",
                Code::KeyB => "B",
                _ => "other",
            });
        }
    })));
}

fn shift(runtime: &mut Runtime) -> bool {
    let mut press = KeyboardEvent::key_down(KeyboardKey::Named(NamedKey::Shift), Code::ShiftLeft);
    press.modifiers = Modifiers::SHIFT;
    runtime.dispatch_keyboard_event(press)
}

fn character(runtime: &mut Runtime, ch: &str, code: Code) -> bool {
    let logical = KeyboardKey::Character(ch.into());
    let handled = runtime.dispatch_keyboard_event(KeyboardEvent::key_down(logical.clone(), code));
    let _ = runtime.dispatch_keyboard_event(KeyboardEvent::key_up(logical, code));
    handled
}

#[test]
fn a_default_manager_and_its_mounted_wrapper_start_with_an_empty_map() {
    let manager = ShortcutManager::new();
    assert!(manager.shortcuts().is_empty());
    assert!(!manager.is_modal());
    let mut runtime = mount(ShortcutScope::manager(
        manager.clone(),
        Focus::new(SizedBox::shrink()).autofocus(true),
    ));
    assert!(runtime.focused_element().is_some());
    assert!(!shift(&mut runtime));
    assert!(manager.shortcuts().is_empty());
}

#[test]
fn a_managed_scope_uses_the_supplied_map_and_reports_the_real_trigger() {
    for external_node in [false, true] {
        let configured = Rc::new(map(KeyboardKey::Named(NamedKey::Shift)));
        let manager = ShortcutManager::with_shortcuts(configured.clone());
        assert!(Rc::ptr_eq(&configured, &manager.shortcuts()));
        let calls = Calls::default();
        observe(&manager, calls.clone());
        let node = FocusNode::new();
        let focused = Focus::new(SizedBox::shrink()).autofocus(true);
        let focused = if external_node {
            focused.node(node.clone())
        } else {
            focused
        };
        let mut runtime = mount(actions(
            ShortcutScope::manager(manager.clone(), focused),
            calls.clone(),
        ));
        assert!(runtime.focused_element().is_some());
        if external_node {
            assert!(node.has_primary_focus());
        }
        assert!(shift(&mut runtime));
        assert_eq!(calls.take(), ["left Shift", "action"]);
        assert!(Rc::ptr_eq(&configured, &manager.shortcuts()));
    }
}

#[test]
fn an_unfocused_manager_does_not_observe_or_invoke_a_binding() {
    let manager = ShortcutManager::with_shortcuts(map(KeyboardKey::Named(NamedKey::Shift)));
    let calls = Calls::default();
    observe(&manager, calls.clone());
    let mut runtime = mount(actions(
        ShortcutScope::manager(manager, SizedBox::shrink()),
        calls.clone(),
    ));
    assert!(runtime.focused_element().is_none());
    assert!(!shift(&mut runtime));
    assert!(calls.borrow().is_empty());
}

#[test]
fn replacing_bindings_does_not_remount_or_change_the_focused_target() {
    let manager = ShortcutManager::with_shortcuts(map(KeyboardKey::Character("a".into())));
    let calls = Calls::default();
    observe(&manager, calls.clone());
    let node = FocusNode::new();
    let mut runtime = mount(actions(
        ShortcutScope::manager(
            manager.clone(),
            Focus::new(SizedBox::shrink())
                .node(node.clone())
                .autofocus(true),
        ),
        calls.clone(),
    ));
    let same_target = runtime.focused_element();
    assert!(character(&mut runtime, "a", Code::KeyA));
    assert_eq!(calls.take(), ["A", "action"]);
    manager.set_shortcuts(map(KeyboardKey::Character("b".into())));
    assert!(!character(&mut runtime, "a", Code::KeyA));
    assert!(character(&mut runtime, "b", Code::KeyB));
    assert_eq!(calls.take(), ["A", "B", "action"]);
    assert_eq!(runtime.focused_element(), same_target);
    assert!(node.has_primary_focus());
}

#[test]
fn a_modal_manager_stops_unmapped_keys_before_the_outer_focus_handler() {
    let manager = ShortcutManager::<u8>::default();
    let calls = Calls::default();
    observe(&manager, calls.clone());
    let inner = ShortcutScope::manager(
        manager.clone(),
        Focus::new(SizedBox::shrink()).autofocus(true),
    );
    let outer = Focus::new(inner).on_key_event({
        let calls = calls.clone();
        move |_, event| {
            if event.state == KeyState::Down {
                calls.borrow_mut().push("outer");
                KeyEventResult::Handled
            } else {
                KeyEventResult::Ignored
            }
        }
    });
    let mut runtime = mount(outer);
    assert!(character(&mut runtime, "a", Code::KeyA));
    assert_eq!(calls.take(), ["A", "outer"]);
    manager.set_modal(true);
    assert!(!character(&mut runtime, "a", Code::KeyA));
    assert_eq!(calls.take(), ["A"]);
}
