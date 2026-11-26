use std::{cell::RefCell, rc::Rc};

use incular_config::Constraints;
use incular_core::{
    Color, InputEvent, Offset, PRIMARY_POINTER_BUTTON, PointerDeviceKind, PointerPhase,
    PointerSampleMetadata, Size, time::Instant,
};
use incular_rendering::PaintCommand;
use incular_runtime::Runtime;
use incular_widgets::{
    ActionInvocationPhase, ActionResult, Center, Command, DismissMenuAction, Focus, FocusNode,
    GestureDetector, Intent, MenuController, RawMenuAnchor, Widget, internal::Key,
};

const PANEL: Color = Color::rgba(29, 76, 81, 255);
fn frame(runtime: &mut Runtime) -> bool {
    runtime
        .run_frame(Constraints::tight(Size::new(800., 600.)))
        .unwrap()
        .0
        .commands()
        .iter()
        .any(|cmd| matches!(cmd, PaintCommand::Rect { color, .. } if *color == PANEL))
}

fn tap(runtime: &mut Runtime, key: Key) {
    let id = runtime
        .tree()
        .element_with_key(&key)
        .expect("live anchor button");
    let bounds = runtime.tree().element_bounds(id).unwrap();
    let position = bounds.origin + Offset::new(bounds.size.width / 2., bounds.size.height / 2.);
    for (phase, buttons) in [
        (PointerPhase::Down, PRIMARY_POINTER_BUTTON),
        (PointerPhase::Up, 0),
    ] {
        let _ = runtime.handle_input_at(
            InputEvent::PointerWithMetadata {
                pointer: 1,
                device: 0,
                kind: PointerDeviceKind::Touch,
                buttons,
                button: Some(PRIMARY_POINTER_BUTTON),
                sample: PointerSampleMetadata::default(),
                phase,
                position,
            },
            Instant::now(),
        );
    }
}

fn exercise_dismiss_action(defer: bool) {
    let controllers: Vec<_> = (0..3).map(|_| MenuController::new()).collect();
    let focus = FocusNode::new();
    let close_requests = Rc::new(RefCell::new(Vec::new()));
    let closes = Rc::new(RefCell::new(Vec::new()));
    let action = DismissMenuAction::new(controllers[2].clone());
    let intent = Intent::<Command>::new(Command::new("dismiss"));
    let registered = action.clone().into_action(intent.clone());
    let invocations = Rc::new(RefCell::new(Vec::new()));
    let _invocation_subscription = registered.add_invocation_listener({
        let log = invocations.clone();
        move |phase| log.borrow_mut().push(phase)
    });
    assert!(!action.is_enabled());
    assert!(!registered.is_action_enabled(&intent));
    assert_eq!(registered.invoke(&intent), ActionResult::Ignored);
    assert!(invocations.borrow().is_empty());

    let mut content = Widget::box_(Size::new(50., 50.), PANEL);
    for level in (0..3).rev() {
        let controller = controllers[level].clone();
        let button = Widget::from(
            GestureDetector::new(Widget::box_(Size::new(80., 30.), Color::WHITE))
                .on_tap(move || controller.open(None)),
        )
        .with_key(Key::Value(level as u64));
        let mut anchor =
            RawMenuAnchor::new(controllers[level].clone(), move |_, _| content.clone())
                .child(if level == 2 {
                    Focus::new(button).node(focus.clone()).into()
                } else {
                    button
                })
                .on_close({
                    let closes = closes.clone();
                    move || closes.borrow_mut().push(level)
                });
        if defer {
            let close_requests = close_requests.clone();
            anchor = anchor.on_close_requested(move |_| close_requests.borrow_mut().push(level));
        }
        content = anchor.into();
    }
    let mut runtime = Runtime::new(Center::new(content).into()).unwrap();
    assert!(!frame(&mut runtime));
    for level in 0..3 {
        tap(&mut runtime, Key::Value(level));
        assert_eq!(frame(&mut runtime), level == 2);
    }
    assert!(controllers.iter().all(MenuController::is_open));
    focus.request_focus();
    assert!(frame(&mut runtime));
    assert!(focus.has_primary_focus());

    assert!(action.is_enabled());
    assert!(registered.is_action_enabled(&intent));
    if defer {
        assert_eq!(*close_requests.borrow(), [1, 2]);
    }
    close_requests.borrow_mut().clear();
    assert_eq!(registered.invoke(&intent), ActionResult::Handled);
    assert_eq!(
        *invocations.borrow(),
        [ActionInvocationPhase::Start, ActionInvocationPhase::End]
    );
    assert_eq!(frame(&mut runtime), defer);
    if defer {
        assert_eq!(*close_requests.borrow(), [0, 1, 2]);
        assert!(closes.borrow().is_empty());
    } else {
        assert_eq!(*closes.borrow(), [2, 1, 0]);
        assert!(
            runtime.tree().element_with_key(&Key::Value(2)).is_none(),
            "closed overlay descendants unmount"
        );
        assert!(
            !action.is_enabled(),
            "unmount must detach the descendant controller"
        );
        assert!(!registered.is_action_enabled(&intent));
    }
}

#[test]
fn dismissing_from_the_focused_submenu_closes_all_ancestors() {
    exercise_dismiss_action(false);
}

#[test]
fn dismissing_from_the_focused_submenu_delivers_every_deferred_request() {
    exercise_dismiss_action(true);
}
