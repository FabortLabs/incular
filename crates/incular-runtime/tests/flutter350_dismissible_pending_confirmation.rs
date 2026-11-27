use std::{cell::RefCell, convert::Infallible, rc::Rc, time::Duration};

use incular_config::Constraints;
use incular_core::{
    DragStartBehavior, InputEvent, Offset, PRIMARY_POINTER_BUTTON, PointerDeviceKind, PointerPhase,
    PointerSampleMetadata, Size, time::Instant,
};
use incular_runtime::Runtime;
use incular_widgets::{
    Directionality, DismissDirection, Dismissible, ListView, Padding, SizedBox, Text,
    TextDirection, UiCompleter, UiFuture, Widget, internal::Key,
};

const SIZE: Size = Size {
    width: 800.,
    height: 600.,
};
type Completion = UiCompleter<Option<bool>, Infallible>;

struct Probe {
    runtime: Runtime,
    complete: Completion,
    seen: Rc<RefCell<Vec<DismissDirection>>>,
    dismissed: Rc<RefCell<Vec<usize>>>,
    next_pointer: u64,
    now: Instant,
    // Keep the descriptor alive after removal; only the mounted instance
    // should receive completion, not a clone retained by application code.
    _descriptor: Widget,
}

impl Probe {
    fn new() -> Self {
        let (future, complete) = UiFuture::pending();
        let seen = Rc::new(RefCell::new(Vec::new()));
        let dismissed = Rc::new(RefCell::new(Vec::new()));
        let rows = (0..9).map(|i| {
            let future = future.clone();
            let seen = seen.clone();
            let dismissed = dismissed.clone();
            let child = SizedBox::new().width(100.).height(100.).child(
                Widget::from(Text::new(i.to_string())).with_key(Key::String(format!("item {i}"))),
            );
            let control = Dismissible::new(DismissDirection::Horizontal, child, move |_| {
                dismissed.borrow_mut().push(i);
            })
            .drag_start_behavior(DragStartBehavior::Down)
            .cross_axis_end_offset(0.5)
            .confirm_dismiss_async(move |direction| {
                seen.borrow_mut().push(direction);
                future.clone()
            });
            Widget::from(control).with_key(i as u64)
        });
        let list = ListView::new(rows)
            .item_extent(100.)
            .drag_start_behavior(DragStartBehavior::Down);
        let descriptor: Widget =
            Directionality::new(TextDirection::Ltr, Padding::all(10., list)).into();
        let runtime = Runtime::new(descriptor.clone()).unwrap();
        let mut probe = Self {
            runtime,
            complete,
            seen,
            dismissed,
            next_pointer: 1,
            now: Instant::now(),
            _descriptor: descriptor,
        };
        probe.frame(0);
        probe
    }

    fn frame(&mut self, advance: u64) {
        self.now += Duration::from_millis(advance);
        self.runtime
            .run_frame_at(Constraints::tight(SIZE), self.now)
            .unwrap();
    }

    fn point(&mut self, phase: PointerPhase, position: Offset, advance: u64) {
        self.now += Duration::from_millis(advance);
        self.runtime.handle_input_at(
            InputEvent::PointerWithMetadata {
                pointer: self.next_pointer,
                device: 0,
                kind: PointerDeviceKind::Touch,
                buttons: if phase == PointerPhase::Up {
                    0
                } else {
                    PRIMARY_POINTER_BUTTON
                },
                button: matches!(phase, PointerPhase::Up | PointerPhase::Down)
                    .then_some(PRIMARY_POINTER_BUTTON),
                sample: PointerSampleMetadata::EMPTY,
                phase,
                position,
            },
            self.now,
        );
    }

    fn settle(&mut self) {
        for delay in [0, 16, 100, 200, 300, 100] {
            self.frame(delay);
        }
    }

    fn slide_pending(&mut self, fling: bool) {
        let content = self
            .runtime
            .tree()
            .element_with_key(&Key::from("item 0"))
            .unwrap();
        let bounds = self.runtime.tree().element_bounds(content).unwrap();
        if fling {
            let from = Offset::new(
                bounds.origin.x + bounds.size.width / 2.,
                bounds.origin.y + 1.,
            );
            self.point(PointerPhase::Down, from, 0);
            for step in 1..=20 {
                self.point(
                    PointerPhase::Move,
                    from + Offset::new(step as f32 * 15., 0.),
                    15,
                );
            }
            self.point(PointerPhase::Up, from + Offset::new(300., 0.), 0);
        } else {
            let from = Offset::new(bounds.origin.x + 0.1, bounds.origin.y + 1.);
            let to = from + Offset::new(bounds.size.width + 0.1, 0.);
            self.point(PointerPhase::Down, from, 0);
            self.point(PointerPhase::Move, to, 50);
            self.point(PointerPhase::Up, to, 50);
        }
        self.next_pointer += 1;
        self.settle();
        assert_eq!(
            *self.seen.borrow(),
            [DismissDirection::StartToEnd],
            "fling={fling}"
        );
        assert!(self.dismissed.borrow().is_empty());
        assert!(self.position().is_some());
    }

    fn position(&self) -> Option<Offset> {
        let tree = self.runtime.tree();
        tree.element_bounds(tree.element_with_key(&Key::from("item 0"))?)
            .map(|b| b.origin)
    }
}

#[test]
fn removed_items_ignore_delayed_true_or_false_after_both_gesture_paths() {
    for fling in [false, true] {
        for answer in [false, true] {
            let mut probe = Probe::new();
            probe.slide_pending(fling);
            let root = probe.runtime.tree().root().unwrap();
            probe
                .runtime
                .schedule_update(
                    root,
                    Directionality::new(TextDirection::Ltr, SizedBox::shrink()).into(),
                )
                .unwrap();
            probe.frame(16);
            assert!(probe.position().is_none());
            assert!(probe.complete.complete(Some(answer)));
            probe.settle();
            assert!(probe.dismissed.borrow().is_empty());
        }
    }
}

#[test]
fn another_drag_cannot_move_the_row_or_start_a_second_confirmation() {
    let mut probe = Probe::new();
    probe.slide_pending(false);
    let position = probe.position().unwrap();
    assert!(
        !probe.runtime.frame_requested(),
        "pending confirmation does not poll on frames"
    );
    let from = Offset::new(100., position.y);
    probe.point(PointerPhase::Down, from, 0);
    probe.point(PointerPhase::Move, from + Offset::new(100., 0.), 100);
    probe.point(PointerPhase::Up, from + Offset::new(100., 0.), 50);
    probe.frame(16);
    assert_eq!(probe.position(), Some(position));
    assert_eq!(*probe.seen.borrow(), [DismissDirection::StartToEnd]);
    assert!(probe.dismissed.borrow().is_empty());
}

#[test]
fn false_or_null_slides_back_to_the_exact_original_position() {
    for answer in [Some(false), None] {
        let mut probe = Probe::new();
        let original = probe.position();
        probe.slide_pending(false);
        assert_ne!(probe.position(), original);
        assert!(probe.complete.complete(answer));
        assert!(
            probe.runtime.frame_requested(),
            "completion wakes the reverse transition"
        );
        probe.settle();
        assert_eq!(probe.position(), original);
        assert!(probe.dismissed.borrow().is_empty());
    }
}

#[test]
fn true_starts_collapse_and_invokes_dismissal_once() {
    let mut probe = Probe::new();
    probe.slide_pending(false);
    assert!(probe.complete.complete(Some(true)));
    assert!(
        probe.runtime.frame_requested(),
        "completion wakes the collapse"
    );
    assert!(!probe.complete.complete(Some(true)));
    probe.settle();
    assert_eq!(*probe.dismissed.borrow(), [0]);
}
