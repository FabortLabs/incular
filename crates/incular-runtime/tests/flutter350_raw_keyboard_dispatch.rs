use std::{cell::RefCell, rc::Rc};

use incular_config::Constraints;
use incular_core::{Code, KeyboardEvent, KeyboardKey, Size};
use incular_runtime::{KeyMessage, Runtime};
use incular_widgets::{FocusNode, RawKeyEvent, RawKeyboardListener, SizedBox};

fn key(code: Code, name: &str, down: bool) -> KeyboardEvent {
    let logical = KeyboardKey::Character(name.into());
    if down {
        KeyboardEvent::key_down(logical, code)
    } else {
        KeyboardEvent::key_up(logical, code)
    }
}

fn dispatch(runtime: &mut Runtime, code: Code, name: &str, down: bool, paired: bool) -> bool {
    let typed = key(code, name, down);
    let raw = RawKeyEvent::from(typed.clone());
    runtime.dispatch_key_message(if paired {
        KeyMessage::TypedAndRaw { typed, raw }
    } else {
        KeyMessage::Raw(raw)
    })
}

fn mount(callback: impl Fn(RawKeyEvent) + 'static) -> Runtime {
    let focus = FocusNode::new();
    let widget = RawKeyboardListener::new(SizedBox::shrink())
        .focus_node(focus.clone())
        .autofocus(true)
        .on_raw_key(callback);
    let mut runtime = Runtime::new(widget.into()).unwrap();
    runtime
        .run_frame(Constraints::tight(Size::new(160., 100.)))
        .unwrap();
    assert!(focus.has_primary_focus());
    runtime
}

#[test]
fn all_raw_handlers_observe_each_message_in_order_with_duplicate_registration() {
    for paired in [false, true] {
        let log = Rc::new(RefCell::new(Vec::new()));
        let observed = log.clone();
        let mut runtime = mount(move |_| observed.borrow_mut().push(1));
        let replay = |runtime: &mut Runtime, down, expected: &[u8]| {
            assert!(!dispatch(runtime, Code::KeyA, "a", down, paired));
            assert_eq!(log.take(), expected);
        };

        replay(&mut runtime, true, &[1]);
        let second = {
            let observed = log.clone();
            move |_: &RawKeyEvent| observed.borrow_mut().push(2)
        };
        let first_registration = runtime.add_raw_keyboard_listener(second.clone());
        replay(&mut runtime, false, &[1, 2]);
        let _third = runtime.add_raw_keyboard_listener({
            let observed = log.clone();
            move |_| observed.borrow_mut().push(3)
        });
        replay(&mut runtime, true, &[1, 2, 3]);
        let _duplicate = runtime.add_raw_keyboard_listener(second);
        replay(&mut runtime, false, &[1, 2, 3, 2]);
        drop(first_registration);
        replay(&mut runtime, true, &[1, 3, 2]);
        replay(&mut runtime, false, &[1, 3, 2]);

        for down in [true, false] {
            assert!(!runtime.dispatch_key_message(KeyMessage::Typed(key(Code::KeyB, "b", down))));
            assert!(
                log.borrow().is_empty(),
                "typed-only messages have no raw counterpart"
            );
        }
    }
}

#[test]
fn caught_raw_listener_failure_reports_original_event_and_does_not_break_releases() {
    let mut runtime = mount(|_| {});
    let failures = Rc::new(RefCell::new(Vec::new()));
    let observed = failures.clone();
    runtime.set_raw_keyboard_error_handler(move |error| observed.borrow_mut().push(error));
    let listener = runtime.add_raw_keyboard_listener(|_| std::panic::panic_any(1_i32));
    let trace = Rc::new(RefCell::new(Vec::new()));
    let observed = trace.clone();
    let _following = runtime.add_raw_keyboard_listener(move |raw| {
        observed.borrow_mut().push(raw.is_key_down());
    });
    assert!(!dispatch(&mut runtime, Code::KeyA, "a", true, false));
    assert!(
        runtime
            .hardware_keyboard()
            .is_physical_key_pressed(Code::KeyA)
    );
    assert_eq!(*trace.borrow(), [true]);
    {
        let failures = failures.borrow();
        assert_eq!(failures.len(), 1);
        assert_eq!(failures[0].payload.downcast_ref::<i32>(), Some(&1));
        assert_eq!(failures[0].event.physical_key(), Code::KeyA);
        assert!(failures[0].event.is_key_down());
    }
    drop(listener);
    failures.borrow_mut().clear();
    assert!(!dispatch(&mut runtime, Code::KeyA, "a", false, false));
    assert!(
        runtime
            .hardware_keyboard()
            .physical_keys_pressed()
            .is_empty()
    );
    assert!(failures.borrow().is_empty());
    assert_eq!(*trace.borrow(), [true, false]);
}

#[test]
fn mounted_raw_observer_panics_are_also_reported_without_stopping_the_window_listener() {
    let mut runtime = mount(|raw| {
        if raw.is_key_down() {
            panic!("mounted raw listener failed");
        }
    });
    let called = Rc::new(RefCell::new(Vec::new()));
    let observer = called.clone();
    runtime.set_raw_keyboard_error_handler(move |error| {
        assert_eq!(error.message(), "mounted raw listener failed");
        observer.borrow_mut().push("reported");
    });
    let observer = called.clone();
    let _following =
        runtime.add_raw_keyboard_listener(move |_| observer.borrow_mut().push("window"));
    assert!(!dispatch(&mut runtime, Code::KeyA, "a", true, true));
    assert!(!dispatch(&mut runtime, Code::KeyA, "a", false, true));
    assert_eq!(*called.borrow(), ["reported", "window", "window"]);
    assert!(
        runtime
            .hardware_keyboard()
            .physical_keys_pressed()
            .is_empty()
    );
}
