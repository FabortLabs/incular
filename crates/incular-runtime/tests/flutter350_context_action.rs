use std::{cell::Cell, rc::Rc};

use incular_config::Constraints;
use incular_core::{Code, KeyboardEvent, KeyboardKey, Modifiers, NamedKey, Size};
use incular_runtime::Runtime;
use incular_widgets::internal::ElementAddress;
use incular_widgets::{
    Action, ActionResult, ActionScope, Actions, Command, Focus, FocusNode, Intent,
    LogicalShortcutKey, ShortcutScope, Shortcuts, SizedBox, Widget,
};

#[derive(Clone)]
struct Marker(&'static str);

#[test]
fn a_context_action_receives_the_invoking_element_and_can_return_no_value() {
    let seen = Rc::new(Cell::new(None));
    let invoked = seen.clone();
    let action = Action::<Command>::with_context("run", move |_, context| {
        let context = context.unwrap();
        assert_eq!(context.find::<Marker>().unwrap().0, "caller");
        assert!(
            invoked
                .replace(context.origin::<ElementAddress>().copied())
                .is_none()
        );
        ActionResult::Handled.into()
    });
    let mut actions = Actions::new();
    actions.register_action(action);
    let dispatcher_seen = Rc::new(Cell::new(None));
    let dispatched = dispatcher_seen.clone();
    let caller = Widget::environment_scope(Marker("caller"), SizedBox::shrink().into());
    let root = Widget::environment_scope(
        Marker("owner"),
        ActionScope::new(Rc::new(actions), caller)
            .dispatcher(Rc::new(move |invocation| {
                dispatched.set(
                    invocation
                        .context()
                        .unwrap()
                        .origin::<ElementAddress>()
                        .copied(),
                );
                assert_eq!(invocation.command::<Command>(), Some(&Command::new("run")));
                invocation.invoke()
            }))
            .into(),
    );
    let mut runtime = Runtime::new(root).unwrap();
    runtime
        .run_frame(Constraints::tight(Size::new(100., 100.)))
        .unwrap();
    let root = runtime.tree().root().unwrap();
    let owner = runtime.tree().children(root).unwrap()[0];
    let caller_proxy = runtime.tree().children(owner).unwrap()[0];
    let caller = runtime.tree().children(caller_proxy).unwrap()[0];
    let address = runtime.tree().element_address(caller);
    let output = runtime
        .invoke_action_output_at(caller, &Intent::<Command>::new("run"))
        .unwrap();
    assert_eq!(output.result(), ActionResult::Handled);
    assert!(output.into_value::<()>().is_none());
    assert_eq!((seen.get(), dispatcher_seen.get()), (address, address));
}

#[test]
fn keyboard_invocation_uses_focused_context_not_the_shortcut_scope() {
    let hit = Rc::new(Cell::new(None));
    let seen = hit.clone();
    let action = Action::<Command>::with_context("run", move |_, context| {
        let context = context.unwrap();
        assert_eq!(context.find::<Marker>().unwrap().0, "focused");
        seen.set(context.origin::<ElementAddress>().copied());
        ActionResult::Handled.into()
    });
    let mut actions = Actions::new();
    actions.register_action(action);
    let mut shortcuts = Shortcuts::new();
    shortcuts.bind_logical(
        LogicalShortcutKey::new(KeyboardKey::Named(NamedKey::Enter), Modifiers::empty()),
        "run",
    );
    let node = FocusNode::new();
    let focus = Focus::new(SizedBox::shrink()).node(node.clone());
    let mut runtime = Runtime::new(
        ActionScope::new(
            Rc::new(actions),
            ShortcutScope::new(
                Rc::new(shortcuts),
                Widget::environment_scope(Marker("focused"), focus.into()),
            ),
        )
        .into(),
    )
    .unwrap();
    let view = Constraints::tight(Size::new(100., 100.));
    runtime.run_frame(view).unwrap();
    node.request_focus();
    runtime.run_frame(view).unwrap();
    assert!(runtime.dispatch_keyboard_event(KeyboardEvent::key_down(
        KeyboardKey::Named(NamedKey::Enter),
        Code::Enter
    )));
    let focused = runtime.tree().focused_keyboard_element().unwrap();
    assert_eq!(hit.get(), runtime.tree().element_address(focused));
}

#[derive(Clone)]
struct Gate(Rc<Cell<bool>>);

#[test]
fn a_context_guard_is_rechecked_by_saved_handlers_and_live_dispatch() {
    let gate = Rc::new(Cell::new(true));
    let count = Rc::new(Cell::new(0));
    let invoked = count.clone();
    let action = Action::<Command>::with_context("run", move |_, context| {
        assert!(context.unwrap().find::<Gate>().unwrap().0.get());
        invoked.set(invoked.get() + 1);
        ActionResult::Handled.into()
    })
    .enabled_when_in_context(|_, context| {
        context
            .and_then(|context| context.find::<Gate>())
            .is_some_and(|gate| gate.0.get())
    });
    let dispatches = Rc::new(Cell::new(0));
    let observed = dispatches.clone();
    let mut actions = Actions::new();
    actions.register_action(action);
    let mut runtime = Runtime::new(
        ActionScope::new(
            Rc::new(actions),
            Widget::environment_scope(Gate(gate.clone()), SizedBox::shrink().into()),
        )
        .dispatcher(Rc::new(move |request| {
            observed.set(observed.get() + 1);
            request.invoke()
        }))
        .into(),
    )
    .unwrap();
    runtime
        .run_frame(Constraints::tight(Size::new(100., 100.)))
        .unwrap();
    let root = runtime.tree().root().unwrap();
    let proxy = runtime.tree().children(root).unwrap()[0];
    let caller = runtime.tree().children(proxy).unwrap()[0];
    let intent = Intent::<Command>::new("run");
    let handler = runtime.tree().action_handler_at(caller, &intent).unwrap();
    handler();
    assert_eq!((count.get(), dispatches.get()), (1, 1));

    gate.set(false);
    assert!(runtime.tree().action_handler_at(caller, &intent).is_none());
    assert_eq!(
        runtime.invoke_action_at(caller, &intent),
        Some(ActionResult::Ignored)
    );
    handler();
    assert_eq!((count.get(), dispatches.get()), (1, 1));

    gate.set(true);
    handler();
    assert_eq!((count.get(), dispatches.get()), (2, 2));
}
