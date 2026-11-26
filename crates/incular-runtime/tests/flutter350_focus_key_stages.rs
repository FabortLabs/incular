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

#[test]
fn root_scope_receives_early_and_late_key_stages_without_a_focused_leaf() {
    use KeyEventResult::{Handled, Ignored, SkipRemainingHandlers};
    let mut runtime = Runtime::new(SizedBox::shrink().into()).unwrap();
    runtime
        .run_frame(Constraints::tight(Size::new(20., 20.)))
        .unwrap();
    assert!(runtime.focused_element().is_none());
    let log = Rc::new(RefCell::new(Vec::new()));
    let early = Rc::new(Cell::new(Handled));
    let late = Rc::new(Cell::new(Handled));
    let handler = |tag, result: Rc<Cell<KeyEventResult>>| {
        let log = log.clone();
        move |event: &KeyboardEvent| {
            if event.state.is_down() {
                log.borrow_mut().push(tag);
                result.get()
            } else {
                Ignored
            }
        }
    };
    let _early = runtime.add_early_focus_key_handler(handler("early", early.clone()));
    let _late = runtime.add_late_focus_key_handler(handler("late", late.clone()));
    let mut dispatch = || {
        let key = KeyboardKey::Character("1".to_owned());
        let consumed =
            runtime.dispatch_keyboard_event(KeyboardEvent::key_down(key.clone(), Code::Digit1));
        assert!(!runtime.dispatch_keyboard_event(KeyboardEvent::key_up(key, Code::Digit1)));
        consumed
    };
    assert!(dispatch());
    assert_eq!(log.take(), ["early"]);
    early.set(Ignored);
    assert!(dispatch());
    assert_eq!(log.take(), ["early", "late"]);
    late.set(SkipRemainingHandlers);
    assert!(!dispatch());
    assert_eq!(log.take(), ["early", "late"]);
}

#[test]
fn focus_key_stages_match_callback_counts_in_raw_and_combined_messages() {
    use KeyEventResult::{Handled, Ignored};
    use incular_runtime::KeyMessage;
    use incular_widgets::RawKeyEvent;

    for raw_only in [true, false] {
        for early_stage in [true, false] {
            let node = FocusNode::new();
            let logs = Rc::new(RefCell::new(Vec::new()));
            let focus_result = Rc::new(Cell::new(Ignored));
            let stage_result = Rc::new(Cell::new(Ignored));
            let scene = Focus::new(SizedBox::shrink())
                .node(node.clone())
                .on_key_event({
                    let logs = logs.clone();
                    let focus_result = focus_result.clone();
                    move |_, event| {
                        logs.borrow_mut().push(0);
                        if event.state.is_down() {
                            focus_result.get()
                        } else {
                            Ignored
                        }
                    }
                })
                .on_key({
                    let logs = logs.clone();
                    move |_, _| {
                        logs.borrow_mut().push(1);
                        Ignored
                    }
                });
            let mut runtime = Runtime::new(scene.into()).unwrap();
            let bounds = Constraints::tight(Size::new(20., 20.));
            runtime.run_frame(bounds).unwrap();
            node.request_focus();
            runtime.run_frame(bounds).unwrap();
            assert!(node.has_primary_focus());

            let key_message = |down| {
                let key = KeyboardKey::Character("1".to_owned());
                let typed = if down {
                    KeyboardEvent::key_down(key, Code::Digit1)
                } else {
                    KeyboardEvent::key_up(key, Code::Digit1)
                };
                let raw = RawKeyEvent::from(typed.clone());
                if raw_only {
                    KeyMessage::Raw(raw)
                } else {
                    KeyMessage::TypedAndRaw { typed, raw }
                }
            };
            let check = |runtime: &mut Runtime, consumed, expected_down: &[u8]| {
                assert_eq!(runtime.dispatch_key_message(key_message(true)), consumed);
                assert_eq!(logs.take(), expected_down);
                assert!(!runtime.dispatch_key_message(key_message(false)));
                assert_eq!(logs.take(), [0, 1]);
            };
            check(&mut runtime, false, &[0, 1]);

            let stage = {
                let stage_result = stage_result.clone();
                move |event: &KeyboardEvent| {
                    if event.state.is_down() {
                        stage_result.get()
                    } else {
                        Ignored
                    }
                }
            };
            let subscription = if early_stage {
                runtime.add_early_focus_key_handler(stage)
            } else {
                runtime.add_late_focus_key_handler(stage)
            };
            stage_result.set(Handled);
            check(&mut runtime, true, if early_stage { &[] } else { &[0, 1] });
            stage_result.set(Ignored);
            check(&mut runtime, false, &[0, 1]);
            focus_result.set(Handled);
            check(&mut runtime, true, &[0, 1]);

            drop(subscription);
            focus_result.set(Ignored);
            stage_result.set(Handled);
            check(&mut runtime, false, &[0, 1]);
            focus_result.set(Handled);
            stage_result.set(Ignored);
            check(&mut runtime, true, &[0, 1]);
        }
    }
}

#[test]
fn typed_only_messages_do_not_invent_raw_data_and_combined_messages_keep_both_views() {
    use incular_runtime::KeyMessage;
    use incular_widgets::RawKeyEvent;
    let calls = Rc::new(RefCell::new(Vec::new()));
    let node = FocusNode::new();
    let scene = Focus::new(SizedBox::shrink())
        .node(node.clone())
        .on_key_event({
            let calls = calls.clone();
            move |_, event| {
                calls.borrow_mut().push((0, event.key));
                KeyEventResult::Ignored
            }
        })
        .on_key({
            let calls = calls.clone();
            move |_, event| {
                calls.borrow_mut().push((1, event.logical_key().clone()));
                KeyEventResult::Ignored
            }
        });
    let mut runtime = Runtime::new(scene.into()).unwrap();
    let bounds = Constraints::tight(Size::new(20., 20.));
    runtime.run_frame(bounds).unwrap();
    node.request_focus();
    runtime.run_frame(bounds).unwrap();
    assert!(node.has_primary_focus());

    let typed_key = KeyboardKey::Character("x".to_owned());
    let raw_key = KeyboardKey::Character("X".to_owned());
    let typed = KeyboardEvent::key_down(typed_key.clone(), Code::KeyX);
    let raw = RawKeyEvent::from(KeyboardEvent::key_down(raw_key.clone(), Code::KeyX));
    assert!(!runtime.dispatch_key_message(KeyMessage::Typed(typed.clone())));
    assert_eq!(calls.take(), [(0, typed_key.clone())]);
    runtime.dispatch_key_message(KeyMessage::Typed(KeyboardEvent::key_up(
        typed_key.clone(),
        Code::KeyX,
    )));
    calls.borrow_mut().clear();
    assert!(!runtime.dispatch_key_message(KeyMessage::TypedAndRaw { typed, raw }));
    assert_eq!(calls.take(), [(0, typed_key), (1, raw_key)]);
}
