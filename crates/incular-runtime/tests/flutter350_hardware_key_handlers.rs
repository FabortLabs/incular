use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use incular_config::Constraints;
use incular_core::{Code, HardwareKeyHandlers, KeyboardEvent, KeyboardKey, Size};
use incular_runtime::Runtime;
use incular_widgets::{FocusNode, KeyboardListener, SizedBox};

fn key(down: bool) -> KeyboardEvent {
    let logical = KeyboardKey::Character("a".to_owned());
    if down {
        KeyboardEvent::key_down(logical, Code::KeyA)
    } else {
        KeyboardEvent::key_up(logical, Code::KeyA)
    }
}

#[test]
fn every_global_handler_runs_in_order_before_focused_notifications() {
    let logs = Rc::new(RefCell::new(Vec::new()));
    let node = FocusNode::new();
    let listener = KeyboardListener::new(SizedBox::shrink())
        .focus_node(node.clone())
        .on_key_event({
            let logs = logs.clone();
            move |_| {
                logs.borrow_mut().push(1);
                false
            }
        });
    let mut runtime = Runtime::new(listener.into()).unwrap();
    let constraints = Constraints::tight(Size::new(20.0, 20.0));
    runtime.run_frame(constraints).unwrap();
    node.request_focus();
    runtime.run_frame(constraints).unwrap();
    assert!(node.has_primary_focus());
    let dispatch = |runtime: &mut Runtime, down, handled, expected: &[i32]| {
        assert_eq!(runtime.dispatch_keyboard_event(key(down)), handled);
        assert_eq!(logs.take(), expected);
    };
    dispatch(&mut runtime, true, false, &[1]);

    let two = Rc::new(Cell::new(false));
    let three = Rc::new(Cell::new(false));
    let callback = |tag, result: &Rc<Cell<bool>>| {
        let (logs, result) = (logs.clone(), result.clone());
        move |_: &KeyboardEvent| {
            logs.borrow_mut().push(tag);
            result.get()
        }
    };
    let second = runtime.add_hardware_key_handler(callback(2, &two));
    dispatch(&mut runtime, false, false, &[2, 1]);
    two.set(true);
    dispatch(&mut runtime, true, true, &[2, 1]);

    two.set(false);
    let _third = runtime.add_hardware_key_handler(callback(3, &three));
    dispatch(&mut runtime, false, false, &[2, 3, 1]);
    two.set(true);
    dispatch(&mut runtime, true, true, &[2, 3, 1]);
    three.set(true);
    dispatch(&mut runtime, false, true, &[2, 3, 1]);

    let _duplicate = runtime.add_hardware_key_handler(callback(2, &two));
    three.set(false);
    two.set(false);
    dispatch(&mut runtime, true, false, &[2, 3, 2, 1]);
    two.set(true);
    dispatch(&mut runtime, false, true, &[2, 3, 2, 1]);
    drop(second);
    dispatch(&mut runtime, true, true, &[3, 2, 1]);
}

#[test]
fn failed_handler_does_not_interrupt_state_or_later_handlers() {
    let dispatcher = HardwareKeyHandlers::default();
    let _bad = dispatcher.add(|_| std::panic::panic_any(1_u8));
    let _good = dispatcher.add(|_| true);
    let report = dispatcher.dispatch(&key(true));
    assert!(report.handled);
    assert_eq!(report.failures.len(), 1);
    assert_eq!(report.failures[0].downcast_ref::<u8>(), Some(&1));

    let mut runtime = Runtime::new(SizedBox::shrink().into()).unwrap();
    let bad = runtime.add_hardware_key_handler(|_| std::panic::panic_any(1_u8));
    assert!(!runtime.dispatch_keyboard_event(key(true)));
    assert!(
        runtime
            .hardware_keyboard()
            .is_physical_key_pressed(Code::KeyA)
    );
    drop(bad);
    assert!(!runtime.dispatch_keyboard_event(key(false)));
    assert!(
        !runtime
            .hardware_keyboard()
            .is_physical_key_pressed(Code::KeyA)
    );
}

#[cfg(panic = "unwind")]
#[test]
fn hardware_key_error_reports_the_original_event_in_both_transit_modes() {
    use incular_runtime::KeyMessage;
    use incular_widgets::RawKeyEvent;

    for raw_only in [true, false] {
        let mut runtime = Runtime::new(SizedBox::shrink().into()).unwrap();
        let reports = Rc::new(RefCell::new(Vec::new()));
        runtime.set_hardware_key_error_handler({
            let reports = reports.clone();
            move |error| {
                reports.borrow_mut().push((
                    error.event.code,
                    error.event.key,
                    error.event.state.is_down(),
                    *error.payload.downcast::<u8>().unwrap(),
                ));
            }
        });
        let bad = runtime.add_hardware_key_handler(|_| std::panic::panic_any(1_u8));
        let later = Rc::new(Cell::new(0));
        let _later = runtime.add_hardware_key_handler({
            let later = later.clone();
            move |_| {
                later.set(later.get() + 1);
                false
            }
        });
        let message = |down| {
            let typed = key(down);
            let raw = RawKeyEvent::from(typed.clone());
            if raw_only {
                KeyMessage::Raw(raw)
            } else {
                KeyMessage::TypedAndRaw { typed, raw }
            }
        };
        assert!(!runtime.dispatch_key_message(message(true)));
        assert_eq!(later.get(), 1);
        assert_eq!(
            reports.take(),
            [(Code::KeyA, KeyboardKey::Character("a".into()), true, 1)]
        );
        assert!(runtime.hardware_keyboard().is_physical_key_pressed(Code::KeyA));
        drop(bad);
        assert!(!runtime.dispatch_key_message(message(false)));
        assert_eq!(later.get(), 2);
        assert!(reports.borrow().is_empty());
        assert!(!runtime.hardware_keyboard().is_physical_key_pressed(Code::KeyA));
    }
}

#[test]
fn mutations_apply_after_the_current_dispatch_snapshot() {
    let dispatcher = HardwareKeyHandlers::default();
    let log = Rc::new(RefCell::new(Vec::new()));
    let removed = Rc::new(RefCell::new(None));
    let added = Rc::new(RefCell::new(None));
    let first = dispatcher.add({
        let (dispatcher, log, removed, added) = (
            dispatcher.clone(),
            log.clone(),
            removed.clone(),
            added.clone(),
        );
        move |_| {
            log.borrow_mut().push(1);
            if removed.borrow_mut().take().is_some() {
                let log = log.clone();
                *added.borrow_mut() = Some(dispatcher.add(move |_| {
                    log.borrow_mut().push(3);
                    false
                }));
            }
            false
        }
    });
    *removed.borrow_mut() = Some(dispatcher.add({
        let log = log.clone();
        move |_| {
            log.borrow_mut().push(2);
            false
        }
    }));
    assert!(!dispatcher.dispatch(&key(true)).handled);
    assert_eq!(log.take(), [1, 2]);
    assert!(!dispatcher.dispatch(&key(false)).handled);
    assert_eq!(log.take(), [1, 3]);
    drop(first);
}

#[cfg(panic = "unwind")]
#[test]
fn key_message_callback_error_preserves_message_and_pressed_state() {
    use incular_runtime::KeyMessage;
    use incular_widgets::RawKeyEvent;

    let mut runtime = Runtime::new(SizedBox::shrink().into()).unwrap();
    let reports = Rc::new(RefCell::new(Vec::new()));
    runtime.set_key_message_error_handler({
        let reports = reports.clone();
        move |error| reports.borrow_mut().push(error)
    });
    let order = Rc::new(RefCell::new(Vec::new()));
    let _hardware = runtime.add_hardware_key_handler({
        let order = order.clone();
        move |_| {
            order.borrow_mut().push("hardware");
            false
        }
    });
    runtime.set_key_message_handler({
        let order = order.clone();
        move |_| {
            order.borrow_mut().push("message");
            std::panic::panic_any(1_i32)
        }
    });
    let down = key(true);
    assert!(!runtime.dispatch_key_message(KeyMessage::TypedAndRaw {
        typed: down.clone(),
        raw: RawKeyEvent::from(down),
    }));
    assert_eq!(order.take(), ["hardware", "message"]);
    let mut errors = reports.take();
    assert_eq!(errors.len(), 1);
    let error = errors.pop().unwrap();
    assert_eq!(error.payload.downcast_ref::<i32>(), Some(&1));
    let KeyMessage::TypedAndRaw { typed, raw } = error.message else {
        panic!("expected the original typed and raw KeyMessage");
    };
    assert_eq!(typed.code, Code::KeyA);
    assert_eq!(typed.key, KeyboardKey::Character("a".into()));
    assert!(typed.state.is_down());
    assert_eq!(raw.physical_key(), Code::KeyA);
    assert_eq!(raw.logical_key(), &typed.key);
    assert!(raw.is_key_down());
    assert!(runtime.hardware_keyboard().is_physical_key_pressed(Code::KeyA));

    runtime.clear_key_message_handler();
    let up = key(false);
    assert!(!runtime.dispatch_key_message(KeyMessage::TypedAndRaw {
        typed: up.clone(),
        raw: up.into(),
    }));
    assert!(reports.borrow().is_empty());
    assert_eq!(order.take(), ["hardware"]);
    assert!(!runtime.hardware_keyboard().is_physical_key_pressed(Code::KeyA));
}

#[test]
fn replacing_and_restoring_key_message_handler_only_changes_focus_delivery() {
    use incular_runtime::KeyMessage;

    let focus = FocusNode::new();
    let focused_calls = Rc::new(Cell::new(0));
    let listener = KeyboardListener::new(SizedBox::shrink())
        .focus_node(focus.clone())
        .on_key_event({
            let focused_calls = focused_calls.clone();
            move |_| {
                focused_calls.set(focused_calls.get() + 1);
                false
            }
        });
    let mut runtime = Runtime::new(listener.into()).unwrap();
    let constraints = Constraints::tight(Size::new(20.0, 20.0));
    runtime.run_frame(constraints).unwrap();
    focus.request_focus();
    runtime.run_frame(constraints).unwrap();
    assert!(focus.has_primary_focus());

    let hardware_handled = Rc::new(Cell::new(true));
    let _hardware = runtime.add_hardware_key_handler({
        let hardware_handled = hardware_handled.clone();
        move |_| hardware_handled.get()
    });
    let messages = Rc::new(RefCell::new(Vec::new()));
    runtime.set_key_message_handler({
        let messages = messages.clone();
        move |message| {
            let seen = match message {
                KeyMessage::TypedAndRaw { typed, raw } => {
                    ("paired", Some(typed.state.is_down()), Some(raw.is_key_down()))
                }
                KeyMessage::Typed(event) => ("typed", Some(event.state.is_down()), None),
                KeyMessage::Raw(raw) => ("raw", None, Some(raw.is_key_down())),
            };
            messages.borrow_mut().push(seen);
            seen.0 == "paired"
        }
    });
    assert!(runtime.dispatch_key_message(KeyMessage::Typed(key(true))));
    hardware_handled.set(false);
    assert!(runtime.dispatch_keyboard_event(key(false)));
    assert!(!runtime.dispatch_key_message(KeyMessage::Typed(key(true))));
    assert!(!runtime.dispatch_key_message(KeyMessage::Raw(key(false).into())));
    assert_eq!(
        messages.take(),
        [
            ("typed", Some(true), None),
            ("paired", Some(false), Some(false)),
            ("typed", Some(true), None),
            ("raw", None, Some(false)),
        ]
    );
    runtime.clear_key_message_handler();
    assert!(!runtime.dispatch_keyboard_event(key(true)));
    assert_eq!(focused_calls.get(), 0);
    runtime.restore_default_key_message_handler();
    assert!(!runtime.dispatch_keyboard_event(key(false)));
    assert_eq!(focused_calls.get(), 1);
}
