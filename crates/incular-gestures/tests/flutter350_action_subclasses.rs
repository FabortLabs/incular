use std::{cell::Cell, rc::Rc};

use incular_gestures::{
    ActionDispatcher, ActionResult, CallbackAction, Intent, VoidCallbackAction, VoidCallbackIntent,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct Command(u32);

#[test]
fn callback_action_receives_the_actual_typed_intent() {
    let received = Rc::new(Cell::new(None));
    let seen = received.clone();
    let action = CallbackAction::new(move |intent: &Intent<Command>| {
        seen.set(Some(*intent.command()));
        true
    });
    let actual = Intent::<Command>::new(Command(81));
    assert!(action.invoke(&actual));
    assert_eq!(received.get(), Some(Command(81)));

    let bound = action.bind(Command(7));
    assert_eq!(
        ActionDispatcher::new().invoke_action(&bound, &actual),
        ActionResult::Ignored
    );
    assert_eq!(
        ActionDispatcher::new().invoke_action(&bound, &Intent::<Command>::new(Command(7))),
        ActionResult::Handled
    );
    assert_eq!(received.get(), Some(Command(7)));
}

#[test]
fn void_callback_action_executes_the_callback_on_the_intent() {
    let ran = Rc::new(Cell::new(0));
    let received = ran.clone();
    let intent = VoidCallbackIntent::new(move || received.set(received.get() + 1));
    let action = VoidCallbackAction;
    assert_eq!(ran.get(), 0);
    action.invoke(&intent);
    assert_eq!(ran.get(), 1);
    action.invoke(&VoidCallbackIntent::new(|| {}));
    assert_eq!(ran.get(), 1);
}
