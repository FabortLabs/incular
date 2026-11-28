//! Shortcuts and app-default no-ops in front of a real focused editor.
use std::{cell::RefCell, rc::Rc};

use incular_config::{Constraints, TextDirection};
use incular_core::{Code, InputEvent, KeyboardEvent, KeyboardKey, Size};
use incular_runtime::Runtime;
use incular_text::TextEditingController;
use incular_widgets::{
    ActionScope, Actions, DoNothingAction, DoNothingAndStopPropagationIntent, DoNothingIntent,
    EditableText, FocusNode, ShortcutActivator, ShortcutScope, Shortcuts, Widget, WidgetsApp,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Command {
    Run,
}

type Calls = Rc<RefCell<Vec<&'static str>>>;

fn shortcut<C: incular_widgets::CommandId>(
    character: &str,
    command: C,
    calls: Option<Calls>,
    child: impl Into<Widget>,
) -> Widget {
    let key = KeyboardKey::Character(character.to_owned());
    let activator = ShortcutActivator::custom(Some(vec![key.clone()]), move |event, _| {
        if event.state.is_down() && event.key == key {
            if let Some(calls) = &calls {
                calls.borrow_mut().push("outer binding");
            }
            true
        } else {
            false
        }
    });
    let mut map = Shortcuts::<C>::typed();
    map.bind_activator(activator, command);
    ShortcutScope::new(Rc::new(map), child).into()
}

fn outer(child: impl Into<Widget>, calls: Calls, bound: bool) -> Widget {
    let shortcuts = shortcut("a", Command::Run, Some(calls.clone()), child);
    if !bound {
        return shortcuts;
    }
    let mut actions = Actions::<Command>::typed();
    actions.register(Command::Run, move || {
        calls.borrow_mut().push("outer action")
    });
    ActionScope::new(Rc::new(actions), shortcuts).into()
}

fn editor(node: &FocusNode, text: &TextEditingController) -> EditableText {
    EditableText::new(text.clone())
        .focus_node(node.clone())
        .text_direction(TextDirection::Ltr)
        .autofocus(true)
        .size(Size::new(180., 45.))
}

fn press_a(root: impl Into<Widget>, node: &FocusNode, text: &TextEditingController, handled: bool) {
    let mut runtime = Runtime::new(root.into()).unwrap();
    runtime
        .run_frame(Constraints::tight(Size::new(180., 100.)))
        .unwrap();
    assert!(node.has_primary_focus());
    let key = KeyboardKey::Character("a".into());
    assert_eq!(
        runtime.dispatch_keyboard_event(KeyboardEvent::key_down(key.clone(), Code::KeyA)),
        handled
    );
    assert!(!runtime.dispatch_keyboard_event(KeyboardEvent::key_up(key, Code::KeyA)));
    assert!(node.has_primary_focus());
    if !handled {
        // A native host delivers printable input separately from key events.
        let _ = runtime.handle_input(InputEvent::Text("a".into()));
        assert_eq!(text.text(), "a");
    } else {
        assert_eq!(text.text(), "");
    }
}

#[test]
fn a_miss_in_the_inner_map_bubbles_to_the_mapped_ancestor() {
    let node = FocusNode::new();
    let text = TextEditingController::new();
    let calls = Calls::default();
    let inner = shortcut("b", DoNothingIntent, None, editor(&node, &text));
    press_a(
        WidgetsApp::new(outer(inner, calls.clone(), true)),
        &node,
        &text,
        true,
    );
    assert_eq!(*calls.borrow(), ["outer binding", "outer action"]);
}

#[test]
fn a_matched_shortcut_without_an_action_leaves_the_key_to_the_text_field() {
    let node = FocusNode::new();
    let text = TextEditingController::new();
    let calls = Calls::default();
    let root = outer(editor(&node, &text), calls.clone(), false);
    press_a(WidgetsApp::new(root), &node, &text, false);
    assert_eq!(*calls.borrow(), ["outer binding"]);
}

#[test]
fn a_matched_action_consumes_the_key_without_unfocusing_the_text_field() {
    let node = FocusNode::new();
    let text = TextEditingController::new();
    let calls = Calls::default();
    let root = outer(editor(&node, &text), calls.clone(), true);
    press_a(WidgetsApp::new(root), &node, &text, true);
    assert_eq!(*calls.borrow(), ["outer binding", "outer action"]);
}

#[test]
fn a_nearer_noop_action_shadows_the_outer_action_but_does_not_consume() {
    let node = FocusNode::new();
    let text = TextEditingController::new();
    let calls = Calls::default();
    let mut nearer = Actions::typed();
    nearer.register_action(DoNothingAction::new(false).bind(Command::Run));
    let edit: Widget = ActionScope::new(Rc::new(nearer), editor(&node, &text)).into();
    press_a(
        WidgetsApp::new(outer(edit, calls.clone(), true)),
        &node,
        &text,
        false,
    );
    assert_eq!(*calls.borrow(), ["outer binding"]);
}

#[test]
fn default_consuming_noop_blocks_the_ancestor_and_the_host() {
    let node = FocusNode::new();
    let text = TextEditingController::new();
    let calls = Calls::default();
    let inner = shortcut("a", DoNothingIntent, None, editor(&node, &text));
    press_a(
        WidgetsApp::new(outer(inner, calls.clone(), true)),
        &node,
        &text,
        true,
    );
    assert!(calls.borrow().is_empty());
}

#[test]
fn default_nonconsuming_noop_blocks_the_ancestor_but_not_native_input() {
    let node = FocusNode::new();
    let text = TextEditingController::new();
    let calls = Calls::default();
    let inner = shortcut(
        "a",
        DoNothingAndStopPropagationIntent,
        None,
        editor(&node, &text),
    );
    press_a(
        WidgetsApp::new(outer(inner, calls.clone(), true)),
        &node,
        &text,
        false,
    );
    assert!(calls.borrow().is_empty());
}

#[test]
fn explicitly_replacing_the_app_action_map_removes_the_default_noops() {
    let node = FocusNode::new();
    let text = TextEditingController::new();
    let calls = Calls::default();
    let inner = shortcut("a", DoNothingIntent, None, editor(&node, &text));
    let app = WidgetsApp::new(outer(inner, calls.clone(), true))
        .actions(Rc::new(Actions::<Command>::typed()));
    press_a(app, &node, &text, true);
    assert_eq!(*calls.borrow(), ["outer binding", "outer action"]);
}
