use std::{cell::RefCell, rc::Rc};

use incular_config::Constraints;
use incular_core::{Code, KeyState, KeyboardEvent, KeyboardKey, Modifiers, NamedKey, Size};
use incular_runtime::Runtime;
use incular_widgets::{
    Action, ActionResult, ActionScope, Actions, Focus, Intent, KeyEventResult, LogicalShortcutKey,
    ScopedActionDispatcher, ShortcutScope, Shortcuts, SizedBox, Widget,
};

const TARGET: u64 = 940;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Command {
    Run,
}

type Calls = Rc<RefCell<Vec<&'static str>>>;

fn record(name: &'static str, calls: &Calls) -> Rc<ScopedActionDispatcher> {
    let calls = calls.clone();
    Rc::new(move |call| {
        assert_eq!(call.command::<Command>(), Some(&Command::Run));
        calls.borrow_mut().push(name);
        call.invoke()
    })
}

#[test]
fn base_action_key_result_follows_its_consumption_policy() {
    let intent: Intent<Command> = Intent::new(Command::Run);
    let action = Action::<Command>::new(Command::Run, || {});
    assert!(action.consumes_key(&intent));
    assert_eq!(action.to_key_event_result(&intent), KeyEventResult::Handled);
    let action = action.with_key_consumption(false);
    assert!(!action.consumes_key(&intent));
    assert_eq!(
        action.to_key_event_result(&intent),
        KeyEventResult::SkipRemainingHandlers
    );
}

fn shortcut_uses_focused_dispatcher(consumes: bool) {
    let calls = Calls::default();
    let action_calls = calls.clone();
    let action = Action::<Command>::new(Command::Run, move || {
        action_calls.borrow_mut().push("action")
    })
    .with_key_consumption(consumes);
    let mut actions = Actions::<Command>::typed();
    actions.register_action(action);

    let child = Widget::from(SizedBox::new().width(20.).height(20.)).with_key(TARGET);
    let child = Focus::new(child).autofocus(true);
    let caller = ActionScope::new(Rc::new(Actions::<Command>::typed()), child)
        .dispatcher(record("caller dispatcher", &calls));
    let mut shortcuts = Shortcuts::<Command>::typed();
    shortcuts.bind_logical(
        LogicalShortcutKey::new(KeyboardKey::Named(NamedKey::Enter), Modifiers::empty()),
        Command::Run,
    );
    let bindings = ShortcutScope::new(Rc::new(shortcuts), caller);
    let owner =
        ActionScope::new(Rc::new(actions), bindings).dispatcher(record("owner dispatcher", &calls));
    let fallback = calls.clone();
    let root = Focus::new(owner).on_key_event(move |_, event| {
        if event.state == KeyState::Down {
            fallback.borrow_mut().push("fallback");
        }
        KeyEventResult::Ignored
    });
    let mut runtime = Runtime::new(root.into()).unwrap();
    runtime
        .run_frame(Constraints::tight(Size::new(100., 100.)))
        .unwrap();
    assert!(runtime.focused_element().is_some());
    let child = runtime.tree().element_with_key(&TARGET.into()).unwrap();

    assert_eq!(
        runtime.invoke_action_at(child, &Intent::<Command>::new(Command::Run)),
        Some(ActionResult::Handled)
    );
    assert_eq!(&*calls.borrow(), &["owner dispatcher", "action"]);
    calls.borrow_mut().clear();

    let enter = KeyboardKey::Named(NamedKey::Enter);
    assert_eq!(
        runtime.dispatch_keyboard_event(KeyboardEvent::key_down(enter.clone(), Code::Enter)),
        consumes
    );
    assert_eq!(&*calls.borrow(), &["caller dispatcher", "action"]);
    let _ = runtime.dispatch_keyboard_event(KeyboardEvent::key_up(enter, Code::Enter));
}

#[test]
fn a_nonconsuming_shortcut_runs_the_caller_dispatcher_and_passes_to_the_os() {
    shortcut_uses_focused_dispatcher(false);
}

#[test]
fn a_consuming_shortcut_runs_the_caller_dispatcher_and_consumes_the_key() {
    shortcut_uses_focused_dispatcher(true);
}
