use std::rc::Rc;

use incular_config::Constraints;
use incular_core::{Code, KeyboardEvent, KeyboardKey, Size};
use incular_runtime::{Application, Runtime, Signal};
use incular_widgets::{Focus, GlobalKey, ShortcutScope, Shortcuts, SizedBox, Widget};

fn bounds() -> Constraints {
    Constraints::tight(Size::new(40., 20.))
}

#[test]
fn an_empty_shortcut_map_mounts_and_does_not_handle_keys() {
    let bindings = Rc::new(Shortcuts::new());
    let root = ShortcutScope::new(
        bindings.clone(),
        Focus::new(SizedBox::shrink()).autofocus(true),
    );
    let mut runtime = Runtime::new(root.into()).unwrap();
    runtime.run_frame(bounds()).unwrap();
    assert!(bindings.is_empty());
    assert!(runtime.focused_element().is_some());
    let key = KeyboardKey::Character("a".into());
    assert!(!runtime.dispatch_keyboard_event(KeyboardEvent::key_down(key.clone(), Code::KeyA)));
    assert!(!runtime.dispatch_keyboard_event(KeyboardEvent::key_up(key, Code::KeyA)));
}

#[test]
fn enabled_shortcut_semantics_creates_an_empty_passive_node_and_can_be_disabled() {
    let include_semantics = Signal::new(true);
    let state = include_semantics.clone();
    let bindings = Rc::new(Shortcuts::new());
    let mut runtime = Application::new(move |_| {
        ShortcutScope::new(bindings.clone(), SizedBox::shrink())
            .include_semantics(state.get())
            .into()
    })
    .unwrap()
    .into_runtime();
    runtime.run_frame(bounds()).unwrap();
    assert!(runtime.focused_element().is_none());
    let semantics = runtime.tree().semantics();
    let [only] = semantics.roots() else {
        panic!("default shortcuts must produce one empty semantic child");
    };
    let node = semantics.node(*only).unwrap();
    assert!(node.label.is_none());
    assert!(node.actions.is_empty());
    assert!(!node.state.focusable && !node.state.focused);
    assert_eq!(semantics.iter().count(), 1);

    include_semantics.set(false);
    runtime.run_frame(bounds()).unwrap();
    assert!(runtime.tree().semantics().is_empty());
    assert!(runtime.focused_element().is_none());
}

#[test]
fn labels_are_attached_to_the_passive_retained_boundary() {
    let label = Signal::new("<My Shortcuts Debug Label>".to_owned());
    let state = label.clone();
    let key = GlobalKey::new();
    let bindings = Rc::new(Shortcuts::new());
    let mut runtime = Application::new(move |_| {
        let body = Widget::from(SizedBox::shrink()).with_key(key);
        ShortcutScope::new(bindings.clone(), body)
            .debug_label(state.get())
            .into()
    })
    .unwrap()
    .into_runtime();
    let node_at = |runtime: &Runtime| {
        let tree = runtime.tree();
        let child = tree.element_with_key(&key.into()).unwrap();
        Focus::of(tree, child, true)
    };
    runtime.run_frame(bounds()).unwrap();
    let first = node_at(&runtime);
    assert_eq!(
        first.debug_label().as_deref(),
        Some("Shortcuts: <My Shortcuts Debug Label>")
    );
    assert!(!first.can_request_focus());
    first.request_focus();
    runtime.run_frame(bounds()).unwrap();
    assert!(runtime.focused_element().is_none());

    label.set("Updated".to_owned());
    runtime.run_frame(bounds()).unwrap();
    let next = node_at(&runtime);
    assert_eq!(next, first);
    assert_eq!(next.debug_label().as_deref(), Some("Shortcuts: Updated"));
    assert!(!next.has_primary_focus());
}
