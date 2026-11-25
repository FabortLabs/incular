use incular_config::Constraints;
use incular_core::{Code, KeyboardEvent, KeyboardKey, Size};
use incular_runtime::Runtime;
use incular_widgets::{Focus, FocusNode, KeyEventResult, SizedBox};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

#[test]
fn early_and_late_focus_key_dispatch_preserves_order_and_native_handling() {
    use KeyEventResult::{Handled, Ignored, SkipRemainingHandlers};
    let key = |down: bool| {
        let logical = KeyboardKey::Character("1".to_owned());
        if down {
            KeyboardEvent::key_down(logical, Code::Digit1)
        } else {
            KeyboardEvent::key_up(logical, Code::Digit1)
        }
    };
    let logs = Rc::new(RefCell::new(Vec::new()));
    let focus = Rc::new(Cell::new(Ignored));
    let early = Rc::new(Cell::new(Ignored));
    let late = Rc::new(Cell::new(Ignored));
    let callback = |tag, result: &Rc<Cell<KeyEventResult>>| {
        let (logs, result) = (logs.clone(), result.clone());
        move |event: &KeyboardEvent| {
            logs.borrow_mut().push(tag);
            if event.state.is_down() {
                result.get()
            } else {
                Ignored
            }
        }
    };
    let node = FocusNode::new();
    let target = callback(2, &focus);
    let mut runtime = Runtime::new(
        Focus::new(SizedBox::shrink())
            .node(node.clone())
            .on_key_event(move |_, event| target(&event))
            .into(),
    )
    .unwrap();
    let constraints = Constraints::tight(Size::new(20., 20.));
    runtime.run_frame(constraints).unwrap();
    node.request_focus();
    runtime.run_frame(constraints).unwrap();
    assert!(node.has_primary_focus());

    let dispatch = |runtime: &mut Runtime, handled, down_order: &[u8], up_order: &[u8]| {
        assert_eq!(runtime.dispatch_keyboard_event(key(true)), handled);
        assert_eq!(logs.take(), down_order);
        assert!(!runtime.dispatch_keyboard_event(key(false)));
        assert_eq!(logs.take(), up_order);
    };
    dispatch(&mut runtime, false, &[2], &[2]);
    let early_handler = runtime.add_early_focus_key_handler(callback(1, &early));
    let late_handler = runtime.add_late_focus_key_handler(callback(3, &late));
    early.set(Handled);
    dispatch(&mut runtime, true, &[1], &[1, 2, 3]);
    early.set(Ignored);
    late.set(Handled);
    dispatch(&mut runtime, true, &[1, 2, 3], &[1, 2, 3]);
    focus.set(Handled);
    dispatch(&mut runtime, true, &[1, 2], &[1, 2, 3]);
    focus.set(SkipRemainingHandlers);
    early.set(SkipRemainingHandlers);
    dispatch(&mut runtime, true, &[1, 2, 3], &[1, 2, 3]);
    late.set(SkipRemainingHandlers);
    dispatch(&mut runtime, false, &[1, 2, 3], &[1, 2, 3]);
    drop(early_handler);
    drop(late_handler);
    focus.set(Ignored);
    dispatch(&mut runtime, false, &[2], &[2]);
}

#[test]
fn existing_snapshot_visits_subscriptions_removed_by_an_earlier_callback() {
    use KeyEventResult::{Handled, Ignored};
    let node = FocusNode::new();
    let focus_calls = Rc::new(Cell::new(0));
    let observed = focus_calls.clone();
    let mut runtime = Runtime::new(
        Focus::new(SizedBox::shrink())
            .node(node.clone())
            .on_key_event(move |_, _| {
                observed.set(observed.get() + 1);
                Ignored
            })
            .into(),
    )
    .unwrap();
    let constraints = Constraints::tight(Size::new(20., 20.));
    runtime.run_frame(constraints).unwrap();
    node.request_focus();
    runtime.run_frame(constraints).unwrap();

    let second = Rc::new(RefCell::new(None));
    let _first = runtime.add_early_focus_key_handler({
        let second = second.clone();
        move |_| {
            second.borrow_mut().take();
            Handled
        }
    });
    let second_calls = Rc::new(Cell::new(0));
    *second.borrow_mut() = Some(runtime.add_early_focus_key_handler({
        let second_calls = second_calls.clone();
        move |_| {
            second_calls.set(second_calls.get() + 1);
            Ignored
        }
    }));
    let logical = || KeyboardKey::Character("1".to_owned());
    assert!(runtime.dispatch_keyboard_event(KeyboardEvent::key_down(logical(), Code::Digit1)));
    assert_eq!(second_calls.get(), 1);
    assert!(runtime.dispatch_keyboard_event(KeyboardEvent::key_up(logical(), Code::Digit1)));
    assert_eq!(second_calls.get(), 1);
    assert_eq!(focus_calls.get(), 0);
}
