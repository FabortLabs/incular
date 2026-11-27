use std::{cell::RefCell, rc::Rc};

use incular_config::Constraints;
use incular_core::Size;
use incular_runtime::Runtime;
use incular_widgets::{
    Action, ActionResult, ActionScope, Actions, Intent, ScopedActionDispatcher, SizedBox, Widget,
};

const KEY: u64 = 931;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
enum Command {
    Run,
}

type Calls = Rc<RefCell<Vec<&'static str>>>;

fn dispatch(name: &'static str, calls: &Calls) -> Rc<ScopedActionDispatcher> {
    let calls = calls.clone();
    Rc::new(move |invocation| {
        assert_eq!(invocation.command::<Command>(), Some(&Command::Run));
        assert_eq!(
            invocation.action::<Command>().unwrap().command(),
            &Command::Run
        );
        calls.borrow_mut().push(name);
        invocation.invoke()
    })
}

fn actions(name: &'static str, calls: &Calls, enabled: bool) -> Rc<Actions<Command>> {
    let calls = calls.clone();
    let action = Action::<Command>::with_handler(Command::Run, move |_| {
        calls.borrow_mut().push(name);
        true
    });
    action.set_enabled(enabled);
    let mut registry = Actions::typed();
    registry.register_action(action);
    Rc::new(registry)
}

fn empty() -> Rc<Actions<Command>> {
    Rc::new(Actions::typed())
}

fn target() -> Widget {
    Widget::from(SizedBox::new().width(10.).height(10.)).with_key(KEY)
}

fn mount(root: ActionScope) -> (Runtime, incular_widgets::internal::ElementId) {
    let mut runtime = Runtime::new(root.into()).unwrap();
    runtime
        .run_frame(Constraints::tight(Size::new(100., 100.)))
        .unwrap();
    let id = runtime.tree().element_with_key(&KEY.into()).unwrap();
    (runtime, id)
}

#[test]
fn a_scope_dispatcher_sees_its_bound_action_and_intent() {
    let calls = Calls::default();
    let root = ActionScope::new(actions("action", &calls, true), target())
        .dispatcher(dispatch("dispatcher", &calls));
    let (mut runtime, element) = mount(root);
    assert_eq!(
        runtime.invoke_action_at(element, &Intent::<Command>::new(Command::Run)),
        Some(ActionResult::Handled)
    );
    assert_eq!(&*calls.borrow(), &["dispatcher", "action"]);
}

fn owner_dispatch(with_inner_dispatcher: bool) {
    let calls = Calls::default();
    let inner = ActionScope::new(empty(), target());
    let inner = if with_inner_dispatcher {
        inner.dispatcher(dispatch("wrong scope", &calls))
    } else {
        inner
    };
    let root = ActionScope::new(actions("outer action", &calls, true), inner)
        .dispatcher(dispatch("outer dispatcher", &calls));
    let (mut runtime, element) = mount(root);
    assert_eq!(
        runtime.invoke_action_at(element, &Intent::<Command>::new(Command::Run)),
        Some(ActionResult::Handled)
    );
    assert_eq!(&*calls.borrow(), &["outer dispatcher", "outer action"]);
}

#[test]
fn a_closer_dispatcher_does_not_intercept_an_ancestor_owned_action() {
    owner_dispatch(true);
}

#[test]
fn an_empty_lower_scope_without_dispatcher_inherits_ancestor_action_dispatch() {
    owner_dispatch(false);
}

#[test]
fn a_dispatcher_lookup_returns_the_registered_dispatcher_instance() {
    let calls = Calls::default();
    let expected = dispatch("nearest", &calls);
    let inner = ActionScope::new(empty(), target()).dispatcher(expected.clone());
    let outer = ActionScope::new(empty(), inner).dispatcher(dispatch("outer", &calls));
    let (runtime, element) = mount(outer);
    let result = runtime.tree().action_dispatcher_at(element).unwrap();
    assert!(Rc::ptr_eq(&result, &expected));
    assert!(calls.borrow().is_empty());
}

#[test]
fn an_action_inherits_its_ancestors_dispatcher_unless_the_mapping_is_disabled() {
    for enabled in [true, false] {
        let calls = Calls::default();
        let inner = ActionScope::new(actions("inner action", &calls, enabled), target());
        let outer = ActionScope::new(actions("outer action", &calls, true), inner)
            .dispatcher(dispatch("outer dispatcher", &calls));
        let (mut runtime, element) = mount(outer);
        let expected = if enabled {
            ActionResult::Handled
        } else {
            ActionResult::Ignored
        };
        assert_eq!(
            runtime.invoke_action_at(element, &Intent::<Command>::new(Command::Run)),
            Some(expected)
        );
        if enabled {
            assert_eq!(&*calls.borrow(), &["outer dispatcher", "inner action"]);
        } else {
            assert!(calls.borrow().is_empty());
        }
    }
}
