//! Flutter `Dismissible` (`widgets/dismissible.dart`): thresholds as a
//! fraction of the child's extent, flings, the slide and resize phases.

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::{Duration, Instant},
};

use incular_config::Constraints;
use incular_core::{Color, Offset, PointerPhase, Size};
use incular_widgets::internal::WidgetTree;
use incular_widgets::{DismissDirection, DismissUpdateDetails, Dismissible, Widget};

struct Harness {
    tree: WidgetTree,
    base: Instant,
    dismissed: Rc<RefCell<Vec<DismissDirection>>>,
    resizes: Rc<Cell<usize>>,
    updates: Rc<RefCell<Vec<DismissUpdateDetails>>>,
}

impl Harness {
    fn new(configure: impl Fn(Dismissible) -> Dismissible) -> Self {
        let dismissed = Rc::new(RefCell::new(Vec::new()));
        let resizes = Rc::new(Cell::new(0));
        let updates = Rc::new(RefCell::new(Vec::new()));
        let dismissible = Dismissible::new(
            DismissDirection::Horizontal,
            Widget::box_(Size::new(120., 80.), Color::WHITE),
            {
                let dismissed = dismissed.clone();
                move |direction| dismissed.borrow_mut().push(direction)
            },
        )
        .on_resize({
            let resizes = resizes.clone();
            move || resizes.set(resizes.get() + 1)
        })
        .on_update({
            let updates = updates.clone();
            move |details| updates.borrow_mut().push(details)
        });
        let mut tree = WidgetTree::new();
        tree.mount(configure(dismissible).into()).expect("mount");
        tree.layout(Constraints::tight(Size::new(120., 80.)))
            .expect("layout");
        Self {
            tree,
            base: Instant::now(),
            dismissed,
            resizes,
            updates,
        }
    }

    fn pointer(&mut self, phase: PointerPhase, x: f32, millis: u64) {
        self.tree.dispatch_gesture(incular_gestures::PointerEvent {
            pointer: 1,
            position: Offset::new(x, 20.),
            phase,
            time: self.base + Duration::from_millis(millis),
        });
    }

    /// A slow drag: the first move only wins the arena (kTouchSlop 18 for
    /// `dispatch_gesture`'s unknown device kind, `DragStartBehavior.start`),
    /// the second moves the child by `extent`, and the release comes long
    /// after the last move, so there is no fling.
    fn slow_drag(&mut self, extent: f32) {
        self.pointer(PointerPhase::Down, 10., 0);
        self.pointer(PointerPhase::Move, 40., 100);
        self.pointer(PointerPhase::Move, 40. + extent, 1_000);
        self.pointer(PointerPhase::Up, 40. + extent, 1_500);
    }

    fn frame(&mut self, millis: u64) {
        self.tree
            .update_compositor(self.base + Duration::from_millis(millis))
            .expect("frame");
        self.tree
            .layout(Constraints::tight(Size::new(120., 80.)))
            .expect("layout");
    }

    fn progress(&self) -> Option<f32> {
        self.updates.borrow().last().map(|details| details.progress)
    }
}

#[test]
fn a_drag_below_the_default_threshold_slides_back() {
    let mut harness = Harness::new(|dismissible| dismissible);
    // 36 / 120 = 0.3, below `_kDismissThreshold` (0.4).
    harness.slow_drag(36.);
    assert_eq!(harness.progress(), Some(0.3));
    for millis in [2_000, 2_030, 2_060, 2_100] {
        harness.frame(millis);
    }
    // The return takes 0.3 x 200 ms (movementDuration) = 60 ms.
    assert_eq!(harness.progress(), Some(0.0));
    assert!(harness.dismissed.borrow().is_empty());
}

#[test]
fn a_drag_past_the_threshold_slides_out_collapses_then_dismisses() {
    let mut harness = Harness::new(|dismissible| dismissible);
    // 60 / 120 = 0.5 > 0.4.
    harness.slow_drag(60.);
    let last = *harness.updates.borrow().last().expect("update");
    assert_eq!(last.direction, DismissDirection::StartToEnd);
    assert!(last.reached);
    // The slide starts on the next frame and takes 0.5 x 200 ms.
    harness.frame(2_000);
    harness.frame(2_050);
    assert!(harness.progress().unwrap() < 1.0);
    harness.frame(2_100);
    assert_eq!(harness.progress(), Some(1.0));
    assert!(harness.dismissed.borrow().is_empty());
    // The 300 ms collapse starts on the following frame.
    harness.frame(2_150);
    harness.frame(2_300);
    assert!(harness.resizes.get() > 0);
    assert!(harness.dismissed.borrow().is_empty());
    harness.frame(2_450);
    assert_eq!(
        harness.dismissed.borrow().as_slice(),
        [DismissDirection::StartToEnd]
    );
}

#[test]
fn without_a_resize_duration_the_dismissal_follows_the_slide() {
    let mut harness = Harness::new(|dismissible| dismissible.resize_duration(None));
    harness.slow_drag(60.);
    harness.frame(2_000);
    harness.frame(2_100);
    assert_eq!(
        harness.dismissed.borrow().as_slice(),
        [DismissDirection::StartToEnd]
    );
    assert_eq!(harness.resizes.get(), 0);
}

#[test]
fn per_direction_thresholds_override_the_default() {
    let mut harness = Harness::new(|dismissible| {
        dismissible.dismiss_threshold(DismissDirection::StartToEnd, 0.6)
    });
    harness.slow_drag(60.);
    for millis in [2_000, 2_100, 2_200] {
        harness.frame(millis);
    }
    assert_eq!(harness.progress(), Some(0.0));
    assert!(harness.dismissed.borrow().is_empty());
}

#[test]
fn a_refused_confirmation_slides_back() {
    let asked = Rc::new(Cell::new(0));
    let mut harness = Harness::new(|dismissible| {
        let asked = asked.clone();
        dismissible.confirm_dismiss(move |_| {
            asked.set(asked.get() + 1);
            false
        })
    });
    harness.slow_drag(60.);
    // Out by 2100 ms; the full 200 ms return starts on the next frame.
    for millis in [2_000, 2_100, 2_150, 2_350] {
        harness.frame(millis);
    }
    assert_eq!(asked.get(), 1);
    assert_eq!(harness.progress(), Some(0.0));
    assert!(harness.dismissed.borrow().is_empty());
}

#[test]
fn a_fast_release_flings_the_child_out() {
    let mut harness = Harness::new(|dismissible| dismissible.resize_duration(None));
    // 20 / 120 is below the threshold, but the release moves at about
    // 2000 px/s, past `_kMinFlingVelocity` (700).
    harness.pointer(PointerPhase::Down, 10., 0);
    harness.pointer(PointerPhase::Move, 40., 10);
    harness.pointer(PointerPhase::Move, 60., 20);
    harness.pointer(PointerPhase::Up, 60., 25);
    for millis in [100, 400] {
        harness.frame(millis);
    }
    assert_eq!(
        harness.dismissed.borrow().as_slice(),
        [DismissDirection::StartToEnd]
    );
}

#[test]
fn end_to_start_ignores_drags_in_the_reading_direction() {
    let dismissed = Rc::new(Cell::new(false));
    let mut harness = Harness::new(|_| {
        let dismissed = dismissed.clone();
        Dismissible::new(
            DismissDirection::EndToStart,
            Widget::box_(Size::new(120., 80.), Color::WHITE),
            move |_| dismissed.set(true),
        )
    });
    harness.slow_drag(60.);
    for millis in [2_000, 2_100, 2_200, 2_600] {
        harness.frame(millis);
    }
    assert!(!dismissed.get());
}
