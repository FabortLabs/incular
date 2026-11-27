use std::{cell::Cell, rc::Rc};

use incular_runtime::Runtime;
use incular_widgets::{
    Action, ActionDispatchInvocation, ActionResult, ActionScope, Actions, Command, Intent, SizedBox,
};

struct Sentinel;

#[test]
fn the_owning_dispatcher_returns_the_exact_application_value_to_the_caller() {
    let sentinel = Rc::new(Sentinel);
    let invoked = Rc::new(Cell::new(false));
    let dispatched = Rc::new(Cell::new(false));
    let command = Command::new("run");
    let intent = Intent::<Command>::new(command.clone());
    let value = sentinel.clone();
    let invocation_seen = invoked.clone();
    let action = Action::<Command>::with_value(command.clone(), move |intent| {
        assert_eq!(intent.command(), &command);
        invocation_seen.set(true);
        value.clone()
    });
    let mut actions = Actions::new();
    actions.register_action(action.clone());
    let dispatcher_called = dispatched.clone();
    let dispatcher = Rc::new(move |request: ActionDispatchInvocation<'_>| {
        dispatcher_called.set(true);
        request.invoke()
    });
    let mut runtime = Runtime::new(
        ActionScope::new(Rc::new(actions), SizedBox::shrink())
            .dispatcher(dispatcher)
            .into(),
    )
    .unwrap();
    let root = runtime.tree().root().unwrap();
    let caller = runtime.tree().children(root).unwrap()[0];
    let output = runtime.invoke_action_output_at(caller, &intent).unwrap();
    assert!(invoked.get() && dispatched.get());
    assert_eq!(output.result(), ActionResult::Handled);
    assert!(Rc::ptr_eq(
        &sentinel,
        output.value::<Rc<Sentinel>>().unwrap()
    ));
    assert!(Rc::ptr_eq(
        &sentinel,
        &output.into_value::<Rc<Sentinel>>().unwrap()
    ));

    invoked.set(false);
    dispatched.set(false);
    action.set_enabled(false);
    let output = runtime.invoke_action_output_at(caller, &intent).unwrap();
    assert_eq!(output.result(), ActionResult::Ignored);
    assert!(output.into_value::<Rc<Sentinel>>().is_none());
    assert!(!invoked.get() && !dispatched.get());
    assert!(
        runtime
            .invoke_action_output_at(caller, &Intent::<Command>::new("absent"))
            .is_none()
    );
}

#[test]
fn a_dispatcher_can_decline_the_action_without_leaking_its_value() {
    let mut actions = Actions::new();
    actions.register_action(Action::with_value("run", |_| 42_i32));
    let mut runtime = Runtime::new(
        ActionScope::new(Rc::new(actions), SizedBox::shrink())
            .dispatcher(Rc::new(|request| {
                assert_eq!(request.invoke(), ActionResult::Handled);
                ActionResult::Ignored
            }))
            .into(),
    )
    .unwrap();
    let caller = runtime.tree().root().unwrap();
    let output = runtime
        .invoke_action_output_at(caller, &Intent::<Command>::new("run"))
        .unwrap();
    assert_eq!(output.result(), ActionResult::Ignored);
    assert!(output.into_value::<i32>().is_none());
}
