use std::{cell::Cell, rc::Rc};

use incular_config::Constraints;
use incular_core::Size;
use incular_runtime::Runtime;
use incular_widgets::{
    Action, ActionDispatcher, ActionResult, ActionScope, Actions, Intent, SizedBox, Widget,
};

const TARGET: u64 = 930;
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum Command {
    Available,
    Missing,
}

fn child() -> Widget {
    Widget::from(SizedBox::new().width(10.).height(10.)).with_key(TARGET)
}

#[test]
fn direct_dispatcher_invokes_with_the_intent_and_returns_the_result() {
    let called = Rc::new(Cell::new(false));
    let seen = called.clone();
    let action = Action::<Command>::with_handler(Command::Available, move |intent| {
        assert_eq!(intent.command(), &Command::Available);
        seen.set(true);
        true
    });
    let result = ActionDispatcher::new().invoke_action(&action, &Intent::new(Command::Available));
    assert_eq!(result, ActionResult::Handled);
    assert!(called.get());
}

fn mounted_dispatch(command: Command, expected: Option<ActionResult>) {
    let called = Rc::new(Cell::new(0));
    let seen = called.clone();
    let mut actions = Actions::<Command>::typed();
    actions.register_handler(Command::Available, move |intent| {
        assert_eq!(intent.command(), &Command::Available);
        seen.set(seen.get() + 1);
        true
    });
    let mut runtime = Runtime::new(ActionScope::new(Rc::new(actions), child()).into()).unwrap();
    runtime
        .run_frame(Constraints::tight(Size::new(100., 100.)))
        .unwrap();
    assert!(runtime.focused_element().is_none());
    let target = runtime.tree().element_with_key(&TARGET.into()).unwrap();
    let result = runtime.invoke_action_at(target, &Intent::<Command>::new(command));
    assert_eq!(result, expected);
    assert_eq!(called.get(), usize::from(expected.is_some()));
}

#[test]
fn action_lookup_uses_the_default_dispatcher_without_requiring_focus() {
    mounted_dispatch(Command::Available, Some(ActionResult::Handled));
}

#[test]
fn optional_mounted_action_handles_registered_commands() {
    mounted_dispatch(Command::Available, Some(ActionResult::Handled));
}

#[test]
fn optional_mounted_action_returns_none_for_missing_commands() {
    mounted_dispatch(Command::Missing, None);
}
