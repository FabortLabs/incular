use std::{cell::Cell, rc::Rc};

use incular_config::Constraints;
use incular_core::{Code, KeyboardEvent, KeyboardKey, Size};
use incular_runtime::Runtime;
use incular_widgets::{
    CallbackShortcuts, Focus, GlobalKey, LogicalKeySet, ShortcutRegistrar, SizedBox, Widget,
};

fn view() -> Constraints {
    Constraints::tight(Size::new(40., 20.))
}

#[test]
fn a_registrar_installs_the_flutter_debug_label_on_its_focus_node() {
    let key = GlobalKey::new();
    let child = Widget::from(SizedBox::shrink()).with_key(key);
    let mut runtime = Runtime::new(ShortcutRegistrar::new(child).into()).unwrap();
    runtime.run_frame(view()).unwrap();
    let tree = runtime.tree();
    let child = tree.element_with_key(&key.into()).unwrap();
    let focus = Focus::of(tree, child, true);
    assert_eq!(
        focus.debug_label().as_deref(),
        Some("Shortcuts: <Shortcut Registrar>")
    );
    assert!(!focus.can_request_focus());
    assert!(runtime.focused_element().is_none());
}

#[test]
fn callback_shortcuts_are_a_passive_focus_boundary_without_blocking_descendants() {
    let key = GlobalKey::new();
    let focus_target_id = GlobalKey::new();
    let focus_target: Widget = Focus::new(SizedBox::shrink()).autofocus(true).into();
    let wrapper_child: Widget = incular_widgets::Column::new([
        Widget::from(SizedBox::shrink()).with_key(key),
        focus_target.with_key(focus_target_id),
    ])
    .into();
    let key_a = KeyboardKey::Character("a".into());
    let hits = Rc::new(Cell::new(0));
    let count = hits.clone();
    let root = CallbackShortcuts::new(wrapper_child).shortcut(
        LogicalKeySet::new([key_a.clone()]),
        move || {
            count.set(count.get() + 1);
        },
    );
    let mut runtime = Runtime::new(root.into()).unwrap();
    runtime.run_frame(view()).unwrap();
    let tree = runtime.tree();
    let child = tree.element_with_key(&key.into()).unwrap();
    let boundary = Focus::of(tree, child, true);
    assert!(!boundary.can_request_focus());
    assert!(boundary.skip_traversal());
    assert!(boundary.has_focus() && !boundary.has_primary_focus());
    assert!(runtime.dispatch_keyboard_event(KeyboardEvent::key_down(key_a.clone(), Code::KeyA)));
    assert!(!runtime.dispatch_keyboard_event(KeyboardEvent::key_up(key_a, Code::KeyA)));
    assert_eq!(hits.get(), 1);
}
