//! KeyboardListener behavior at the normalized host-event boundary, against
//! pinned Flutter 3.50.0 widgets/keyboard_listener_test.dart.
//! This does not emulate browser or Fuchsia raw-key normalization.

use std::{cell::RefCell, rc::Rc};

use incular_config::{Alignment, Constraints};
use incular_core::{
    Code, InputEvent, KeyState, KeyboardEvent, KeyboardKey, Location, NamedKey, Size,
};
use incular_platform::PlatformEvent;
use incular_runtime::Runtime;
use incular_widgets::{Align, FocusNode, KeyboardListener, SizedBox, Widget};

type Events = Rc<RefCell<Vec<KeyboardEvent>>>;

fn frame(runtime: &mut Runtime) {
    runtime
        .run_frame(Constraints::tight(Size::new(200., 100.)))
        .unwrap();
}

fn shell(child: impl Into<Widget>) -> Widget {
    Align::new(Alignment::CENTER, child).into()
}

fn update(runtime: &mut Runtime, root: impl Into<Widget>) {
    let old = runtime.tree().root().unwrap();
    runtime.schedule_update(old, shell(root)).unwrap();
    frame(runtime);
}

fn send_meta_left(runtime: &mut Runtime) {
    for state in [KeyState::Down, KeyState::Up] {
        let mut key = KeyboardEvent::key_down(KeyboardKey::Named(NamedKey::Meta), Code::MetaLeft);
        key.state = state;
        key.location = Location::Left;
        let _ = runtime.handle_platform_event(PlatformEvent::Input(InputEvent::Key(key)));
    }
    frame(runtime);
}

fn focused_listener() -> (Runtime, FocusNode, Events) {
    let node = FocusNode::new();
    let events = Events::default();
    let listener = KeyboardListener::new(SizedBox::shrink())
        .focus_node(node.clone())
        .on_key_event({
            let events = events.clone();
            move |event| {
                events.borrow_mut().push(event);
                false
            }
        });
    let mut runtime = Runtime::new(shell(listener)).unwrap();
    frame(&mut runtime);
    node.request_focus();
    frame(&mut runtime);
    assert!(node.has_primary_focus());
    (runtime, node, events)
}

// Upstream :10.
#[test]
fn keyboard_listener_can_be_rebuilt_and_unmounted_without_any_key_input() {
    let node = FocusNode::new();
    let make_listener = || KeyboardListener::new(SizedBox::shrink()).focus_node(node.clone());
    let mut runtime = Runtime::new(shell(make_listener())).unwrap();
    frame(&mut runtime);
    update(&mut runtime, make_listener());
    update(&mut runtime, SizedBox::shrink());
    assert!(runtime.focused_element().is_none());
    assert!(!node.has_primary_focus());
}

// Upstream :42, normalized event-delivery subset.
#[test]
fn focused_listener_receives_meta_down_and_up_with_left_physical_identity() {
    let (mut runtime, _node, events) = focused_listener();
    send_meta_left(&mut runtime);

    let events = events.borrow();
    assert_eq!(events.len(), 2);
    assert_eq!(events[0].state, KeyState::Down);
    assert_eq!(events[0].code, Code::MetaLeft);
    assert_eq!(events[0].key, KeyboardKey::Named(NamedKey::Meta));
    assert_eq!(events[0].location, Location::Left);
    assert_eq!(events[1].state, KeyState::Up);
}

// Upstream :66, supplying normalized keys instead of Fuchsia raw data.
#[test]
fn a_removed_keyboard_listener_no_longer_receives_meta_key_events() {
    let (mut runtime, _node, events) = focused_listener();
    send_meta_left(&mut runtime);
    assert_eq!(events.take().len(), 2);

    update(&mut runtime, SizedBox::shrink());
    assert!(runtime.focused_element().is_none());
    send_meta_left(&mut runtime);
    assert!(events.borrow().is_empty());
}

#[test]
fn raw_keyboard_listener_has_an_unfocused_focusable_node_by_default() {
    use incular_semantics::SemanticActionKind;
    use incular_widgets::RawKeyboardListener;

    let node = FocusNode::new();
    let mut runtime = Runtime::new(
        RawKeyboardListener::new(SizedBox::expand())
            .focus_node(node)
            .into(),
    )
    .unwrap();
    frame(&mut runtime);
    let tree = runtime.tree().semantics();
    let [id] = tree.roots() else {
        panic!("expected one listener semantic node");
    };
    let node = tree.node(*id).unwrap();
    assert!(node.state.focusable);
    assert!(!node.state.focused);
    assert!(node.label.is_none());
    assert!(node.children.is_empty());
    assert_eq!(node.actions, [SemanticActionKind::Focus]);
    assert_eq!(tree.len(), 1);
}

#[test]
fn raw_keyboard_listener_can_hide_semantics_without_losing_keyboard_focus_or_events() {
    use incular_widgets::RawKeyboardListener;

    let node = FocusNode::new();
    let events = Events::default();
    let listener = RawKeyboardListener::new(SizedBox::expand())
        .focus_node(node.clone())
        .include_semantics(false)
        .on_key({
            let events = events.clone();
            move |key| events.borrow_mut().push(key)
        });
    let mut runtime = Runtime::new(listener.into()).unwrap();
    frame(&mut runtime);
    assert!(runtime.tree().semantics().roots().is_empty());
    assert!(runtime.tree().semantics().is_empty());
    node.request_focus();
    frame(&mut runtime);
    assert!(node.has_primary_focus());
    send_meta_left(&mut runtime);
    assert_eq!(
        events
            .borrow()
            .iter()
            .map(|key| key.state)
            .collect::<Vec<_>>(),
        [KeyState::Down, KeyState::Up]
    );
    assert!(runtime.tree().semantics().is_empty());
}
