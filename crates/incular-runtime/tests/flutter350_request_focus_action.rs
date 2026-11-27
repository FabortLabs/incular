use std::{cell::Cell, rc::Rc};

use incular_config::Constraints;
use incular_core::{Code, KeyboardEvent, KeyboardKey, Modifiers, NamedKey, Size};
use incular_runtime::Runtime;
use incular_widgets::{
    Action, ActionResult, ActionScope, Actions, Column, Focus, FocusNode, Intent,
    RequestFocusAction, RequestFocusIntent, ScrollController, Shortcuts, SingleActivator,
    SingleChildScrollView, SizedBox, Widget, WidgetsApp, internal::Key,
};

const CALLER: Key = Key::Value(38491);

fn content(node: &FocusNode, scroll: &ScrollController) -> Widget {
    let target =
        Widget::from(Focus::new(SizedBox::new().height(60.)).node(node.clone())).with_key(CALLER);
    SingleChildScrollView::new(Column::new([
        SizedBox::new().height(900.).into(),
        target,
        SizedBox::new().height(400.).into(),
    ]))
    .controller(scroll.clone())
    .into()
}

fn frame(runtime: &mut Runtime) {
    runtime
        .run_frame(Constraints::tight(Size::new(800., 600.)))
        .unwrap();
}

#[test]
fn default_request_focus_action_and_intent_callback_are_exclusive() {
    let node = FocusNode::new();
    let scroll = ScrollController::new();
    let mut runtime = Runtime::new(content(&node, &scroll)).unwrap();
    frame(&mut runtime);
    assert!(!node.has_focus());

    RequestFocusAction.invoke(runtime.tree(), &RequestFocusIntent::new(node.clone()));
    frame(&mut runtime);
    assert!(node.has_focus());
    // Target ends at 960 and the viewport is 600px high: use alignment
    // 1.0 rather than scrolling its leading edge to the top.
    assert_eq!(scroll.offset(), 360.);

    node.unfocus();
    assert!(scroll.jump_to(0.));
    frame(&mut runtime);
    assert!(!node.has_focus());
    let calls = Rc::new(Cell::new(0));
    let intent = RequestFocusIntent::new(node.clone()).request_focus_callback({
        let calls = calls.clone();
        let expected = node.clone();
        move |target| {
            assert_eq!(target, &expected);
            calls.set(calls.get() + 1);
        }
    });
    RequestFocusAction.invoke(runtime.tree(), &intent);
    frame(&mut runtime);
    assert_eq!(calls.get(), 1);
    assert!(!node.has_focus());
    assert_eq!(scroll.offset(), 0.);
}

#[test]
fn widgets_app_resolves_default_and_local_replacements_without_extra_focus_or_scroll() {
    let node = FocusNode::new();
    let scroll = ScrollController::new();
    let command = RequestFocusIntent::new(node.clone());
    let mut runtime = Runtime::new(WidgetsApp::new(content(&node, &scroll)).into()).unwrap();
    frame(&mut runtime);
    assert!(scroll.jump_to(450.));
    frame(&mut runtime);
    // The entire target is already visible. Explicit alignment still
    // moves it to the trailing edge, unlike a keep-visible policy.
    let id = runtime.tree().element_with_key(&CALLER).unwrap();
    let intent = Intent::from(command.clone());
    assert_eq!(
        runtime.invoke_action_at(id, &intent),
        Some(ActionResult::Handled)
    );
    frame(&mut runtime);
    assert!(node.has_focus());
    assert_eq!(scroll.offset(), 360.);

    let node = FocusNode::new();
    let command = RequestFocusIntent::new(node.clone());
    let replacement_calls = Rc::new(Cell::new(0));
    let action = Action::new(command.clone(), {
        let replacement_calls = replacement_calls.clone();
        move || replacement_calls.set(replacement_calls.get() + 1)
    });
    let mut actions = Actions::typed();
    actions.register_action(action.clone());
    let other_scroll = ScrollController::new();
    let local = ActionScope::new(Rc::new(actions), content(&node, &other_scroll));
    drop(runtime);
    let mut runtime = Runtime::new(WidgetsApp::new(local).into()).unwrap();
    frame(&mut runtime);
    let id = runtime.tree().element_with_key(&CALLER).unwrap();
    assert!(!node.has_focus());
    let intent = Intent::from(command.request_focus_callback(|_| {
        panic!("the local action, not the default intent callback, owns dispatch")
    }));
    assert_eq!(
        runtime.invoke_action_at(id, &intent),
        Some(ActionResult::Handled)
    );
    action.set_enabled(false);
    assert_eq!(
        runtime.invoke_action_at(id, &intent),
        Some(ActionResult::Ignored)
    );
    frame(&mut runtime);
    assert_eq!(replacement_calls.get(), 1);
    assert!(!node.has_focus());
    assert_eq!(other_scroll.offset(), 0.);
}

#[test]
fn a_bound_shortcut_uses_widgets_apps_request_focus_default() {
    let source = FocusNode::new();
    let target = FocusNode::new();
    let scroll = ScrollController::new();
    let view = SingleChildScrollView::new(Column::new([
        Widget::from(Focus::new(SizedBox::new().height(900.)).node(source.clone())),
        Focus::new(SizedBox::new().height(60.))
            .node(target.clone())
            .into(),
        SizedBox::new().height(400.).into(),
    ]))
    .controller(scroll.clone());
    let key = KeyboardKey::Named(NamedKey::F8);
    let mut shortcuts = Shortcuts::typed();
    shortcuts.bind_activator(
        SingleActivator::new(key.clone(), Modifiers::empty()),
        RequestFocusIntent::new(target.clone()),
    );
    let app = WidgetsApp::new(view).shortcuts(Rc::new(shortcuts));
    let mut runtime = Runtime::new(app.into()).unwrap();
    frame(&mut runtime);
    source.request_focus();
    frame(&mut runtime);
    assert!(source.has_primary_focus());
    assert!(runtime.dispatch_keyboard_event(KeyboardEvent::key_down(key, Code::F8)));
    frame(&mut runtime);
    assert!(!source.has_focus());
    assert!(target.has_primary_focus());
    assert_eq!(scroll.offset(), 360.);
}
