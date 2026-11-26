//! Actual pointer routing through custom background and foreground painters.
use std::{
    cell::Cell,
    rc::Rc,
    time::{Duration, Instant},
};

use incular_config::{Constraints, TextDirection};
use incular_core::{Offset, PointerPhase, Size};
use incular_gestures::PointerEvent;
use incular_rendering::DisplayList;
use incular_widgets::{
    CustomPaint, CustomPainter, Directionality, GestureDetector, HitTestBehavior, SizedBox, Stack,
    Widget, internal::WidgetTree,
};

const SIZE: Size = Size {
    width: 100.,
    height: 100.,
};

#[derive(Clone)]
enum HitPolicy {
    Default,
    Decline,
    NearEdge(Rc<Cell<Option<Offset>>>),
}

impl CustomPainter for HitPolicy {
    fn paint(&self, _: Size) -> DisplayList {
        DisplayList::default()
    }

    fn should_repaint(&self, _: &Self) -> bool {
        false
    }

    fn hit_test(&self, point: Offset) -> Option<bool> {
        match self {
            Self::Default => None,
            Self::Decline => Some(false),
            Self::NearEdge(observed) => {
                observed.set(Some(point));
                Some(point.x < 40.)
            }
        }
    }
}

fn click(tree: &mut WidgetTree, point: Offset) {
    let start = Instant::now();
    for (phase, elapsed) in [(PointerPhase::Down, 0), (PointerPhase::Up, 15)] {
        tree.dispatch_gesture(PointerEvent {
            pointer: 1,
            position: point,
            phase,
            time: start + Duration::from_millis(elapsed),
        });
    }
}

fn button(calls: Rc<Cell<usize>>) -> GestureDetector {
    GestureDetector::new(SizedBox::new().width(SIZE.width).height(SIZE.height))
        .behavior(HitTestBehavior::Opaque)
        .on_tap(move || calls.set(calls.get() + 1))
}

fn layout(tree: &mut WidgetTree) {
    tree.layout(Constraints::tight(SIZE)).unwrap();
    tree.update_compositor(Instant::now()).unwrap();
}

#[test]
fn background_accepts_by_default_and_can_decline_the_hit() {
    let calls = Rc::new(Cell::new(0));
    let button: Widget = button(calls.clone()).into();
    let scene = |policy| {
        Widget::from(Directionality::new(
            TextDirection::Ltr,
            Stack::new([
                button.clone(),
                CustomPaint::from_painter(SIZE, policy).into(),
            ]),
        ))
    };
    let mut tree = WidgetTree::new();
    let root = tree.mount(scene(HitPolicy::Default)).unwrap();
    layout(&mut tree);
    click(&mut tree, Offset::new(20., 20.));
    assert_eq!(calls.get(), 0, "background claims the tap");

    tree.update(root, scene(HitPolicy::Decline)).unwrap();
    layout(&mut tree);
    click(&mut tree, Offset::new(20., 20.));
    assert_eq!(
        calls.get(),
        1,
        "declined tap reaches sibling behind the painter"
    );
}

#[test]
fn foreground_passes_through_by_default_and_can_claim_before_its_child() {
    let calls = Rc::new(Cell::new(0));
    let local_seen = Rc::new(Cell::new(None));
    let target: Widget = button(calls.clone()).into();
    let scene = |policy| {
        CustomPaint::new(SIZE, DisplayList::default())
            .foreground_painter(policy)
            .child(target.clone())
            .into()
    };
    let mut tree = WidgetTree::new();
    let root = tree.mount(scene(HitPolicy::Default)).unwrap();
    layout(&mut tree);
    click(&mut tree, Offset::new(20., 20.));
    assert_eq!(calls.get(), 1);

    tree.update(root, scene(HitPolicy::NearEdge(local_seen.clone())))
        .unwrap();
    layout(&mut tree);
    click(&mut tree, Offset::new(20., 20.));
    assert_eq!(local_seen.get(), Some(Offset::new(20., 20.)));
    assert_eq!(calls.get(), 1, "foreground preempts its child");
    click(&mut tree, Offset::new(60., 20.));
    assert_eq!(local_seen.get(), Some(Offset::new(60., 20.)));
    assert_eq!(calls.get(), 2, "a different point still reaches the child");
}
