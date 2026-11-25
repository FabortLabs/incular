use std::{cell::Cell, collections::HashSet, rc::Rc};

use incular_config::Constraints;
use incular_core::{Code, KeyboardEvent, KeyboardKey, Modifiers, NamedKey, Size};
use incular_runtime::Runtime;
use incular_widgets::{CallbackShortcuts, Focus, SingleActivator, SizedBox};

struct Harness {
    runtime: Runtime,
    held: HashSet<Code>,
    hits: Rc<Cell<usize>>,
}

impl Harness {
    fn new(activator: SingleActivator) -> Self {
        let hits = Rc::new(Cell::new(0));
        let callbacks = CallbackShortcuts::new(Focus::new(SizedBox::shrink()).autofocus(true))
            .shortcut(activator, {
                let hits = hits.clone();
                move || hits.set(hits.get() + 1)
            });
        let mut runtime = Runtime::new(callbacks.into()).unwrap();
        runtime
            .run_frame(Constraints::tight(Size::new(20.0, 20.0)))
            .unwrap();
        assert!(runtime.focused_element().is_some());
        Self {
            runtime,
            held: HashSet::new(),
            hits,
        }
    }

    fn keys(&mut self, keys: &[(Code, bool)]) {
        for &(code, down) in keys {
            let key = match code {
                Code::ControlLeft | Code::ControlRight => KeyboardKey::Named(NamedKey::Control),
                Code::ShiftLeft => KeyboardKey::Named(NamedKey::Shift),
                Code::KeyA => KeyboardKey::Character("a".to_owned()),
                Code::KeyC => KeyboardKey::Character("c".to_owned()),
                _ => unreachable!("the fixture only generates the selected keys"),
            };
            let mut event = if down {
                self.held.insert(code);
                KeyboardEvent::key_down(key, code)
            } else {
                self.held.remove(&code);
                KeyboardEvent::key_up(key, code)
            };
            event.modifiers.set(
                Modifiers::CONTROL,
                self.held.contains(&Code::ControlLeft) || self.held.contains(&Code::ControlRight),
            );
            event
                .modifiers
                .set(Modifiers::SHIFT, self.held.contains(&Code::ShiftLeft));
            let _ = self.runtime.dispatch_keyboard_event(event);
        }
    }

    fn count(&self, expected: usize) {
        assert_eq!(self.hits.get(), expected);
    }

    fn reset_count(&self) {
        self.hits.set(0);
    }

    fn repeat_c(&mut self) {
        assert!(self.held.contains(&Code::ControlLeft));
        assert!(self.held.contains(&Code::KeyC));
        let mut event = KeyboardEvent::key_down(KeyboardKey::Character("c".to_owned()), Code::KeyC);
        event.repeat = true;
        event.modifiers = Modifiers::CONTROL;
        let _ = self.runtime.dispatch_keyboard_event(event);
    }

    fn assert_released(&self) {
        assert!(self.held.is_empty());
        assert!(
            self.runtime
                .hardware_keyboard()
                .logical_keys_pressed()
                .is_empty()
        );
    }
}

fn c_with(modifiers: Modifiers) -> SingleActivator {
    SingleActivator::new(KeyboardKey::Character("c".to_owned()), modifiers)
}

#[test]
fn ctrl_c_matches_only_the_trigger_down_with_exact_modifiers() {
    use Code::{ControlLeft as L, ControlRight as R, KeyA as A, KeyC as C, ShiftLeft as S};

    let mut h = Harness::new(c_with(Modifiers::CONTROL));
    let d = |code| (code, true);
    let u = |code| (code, false);

    h.keys(&[d(L)]);
    h.count(0);
    h.keys(&[d(C)]);
    h.count(1);
    h.keys(&[u(C), u(L)]);
    h.count(1);
    h.reset_count();

    h.keys(&[d(C), d(L)]);
    h.count(0);
    h.keys(&[u(C), u(L)]);
    h.reset_count();

    h.keys(&[d(S), d(L), d(C)]);
    h.count(0);
    h.keys(&[u(C), u(L), u(S)]);
    h.reset_count();

    h.keys(&[d(L), d(C)]);
    h.reset_count();
    h.keys(&[d(A), u(A)]);
    h.count(0);
    h.keys(&[u(C), u(L)]);
    h.reset_count();

    h.keys(&[d(L), d(A)]);
    h.count(0);
    h.keys(&[d(C)]);
    h.count(1);
    h.keys(&[u(A), u(C), u(L)]);
    h.reset_count();

    h.keys(&[d(R)]);
    h.count(0);
    h.keys(&[d(C)]);
    h.count(1);
    h.keys(&[u(C), u(R)]);
    h.count(1);
    h.reset_count();

    h.keys(&[d(L), d(R)]);
    h.count(0);
    h.keys(&[d(C)]);
    h.count(1);
    h.keys(&[u(C), u(L), u(R)]);
    h.count(1);
    h.reset_count();

    h.keys(&[d(L)]);
    h.count(0);
    h.keys(&[d(C)]);
    h.count(1);
    h.keys(&[d(A)]);
    h.count(1);
    h.keys(&[u(C), u(A), u(L)]);
    h.count(1);
    h.assert_released();
}

#[test]
fn shift_ctrl_c_accepts_either_modifier_order_but_not_a_late_shift() {
    use Code::{ControlLeft as L, KeyC as C, ShiftLeft as S};

    let mut h = Harness::new(c_with(Modifiers::CONTROL | Modifiers::SHIFT));
    let d = |code| (code, true);
    let u = |code| (code, false);
    for (first, second) in [(S, L), (L, S)] {
        h.keys(&[d(first), d(second)]);
        h.count(0);
        h.keys(&[d(C)]);
        h.count(1);
        h.keys(&[u(C), u(L), u(S)]);
        h.count(1);
        h.reset_count();
    }
    h.keys(&[d(L), d(C), d(S)]);
    h.count(0);
    h.keys(&[u(C), u(L), u(S)]);
    h.count(0);
    h.assert_released();
}

fn check_ctrl_c_repeats(activator: SingleActivator, expected_after_repeat: usize) {
    use Code::{ControlLeft as L, KeyC as C};

    let mut h = Harness::new(activator);
    h.keys(&[(L, true)]);
    h.count(0);
    h.keys(&[(C, true)]);
    h.count(1);
    h.repeat_c();
    h.count(expected_after_repeat);
    h.keys(&[(C, false), (L, false)]);
    h.count(expected_after_repeat);
    h.assert_released();
}

#[test]
fn ctrl_c_repeats_by_default() {
    check_ctrl_c_repeats(c_with(Modifiers::CONTROL), 2);
}

#[test]
fn ctrl_c_can_reject_repeats() {
    check_ctrl_c_repeats(c_with(Modifiers::CONTROL).with_repeats(false), 1);
}
