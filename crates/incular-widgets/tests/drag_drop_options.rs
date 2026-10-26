//! Flutter `Draggable`/`DragTarget` options and defaults
//! (`widgets/drag_target.dart`).

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::{Duration, Instant},
};

use incular_config::{Axis, Constraints, TextDirection};
use incular_core::{Color, Offset, PointerPhase, Size};
use incular_widgets::internal::WidgetTree;
use incular_widgets::{
    DragAnchorStrategy, DragDropContext, DragTarget, Draggable, DraggableDetails, Row, Widget,
};

fn card() -> Widget {
    Widget::box_(Size::new(100., 80.), Color::WHITE)
}

/// A draggable card at the origin and a target card to its right.
fn mount(draggable: Draggable<u32>, target: DragTarget<u32>) -> WidgetTree {
    let mut tree = WidgetTree::new();
    tree.mount(
        Row::new(vec![Widget::from(draggable), Widget::from(target)])
            .text_direction(TextDirection::Ltr)
            .into(),
    )
    .expect("mount");
    tree.layout(Constraints::tight(Size::new(200., 80.)))
        .expect("layout");
    tree
}

fn drive(tree: &mut WidgetTree, events: &[(PointerPhase, f32, f32)]) {
    let base = Instant::now();
    for (index, (phase, x, y)) in events.iter().enumerate() {
        tree.dispatch_gesture(incular_gestures::PointerEvent {
            pointer: 1,
            position: Offset::new(*x, *y),
            phase: *phase,
            time: base + Duration::from_millis(index as u64 * 10),
        });
    }
}

#[test]
fn drags_start_past_the_touch_slop_in_any_direction() {
    // `ImmediateMultiDragGestureRecognizer`: the displacement must exceed
    // computeHitSlop (kTouchSlop 18 for the unknown device kind).
    let context = DragDropContext::<u32>::new();
    let mut tree = mount(
        Draggable::new(context.clone(), 1, card()),
        DragTarget::new(context.clone(), card()),
    );
    drive(
        &mut tree,
        &[
            (PointerPhase::Down, 10., 20.),
            (PointerPhase::Move, 28., 20.),
        ],
    );
    assert!(!context.is_dragging(), "18 px is not past the slop");
    drive(&mut tree, &[(PointerPhase::Move, 22., 34.)]);
    // (12, 14) from the press: 18.44 px.
    assert!(context.is_dragging());
}

#[test]
fn child_anchor_keeps_the_feedback_over_the_child() {
    let context = DragDropContext::<u32>::new();
    let mut tree = mount(
        Draggable::new(context.clone(), 1, card()).feedback(|_| card()),
        DragTarget::new(context.clone(), card()),
    );
    drive(
        &mut tree,
        &[
            (PointerPhase::Down, 10., 20.),
            (PointerPhase::Move, 40., 20.),
        ],
    );
    // `childDragAnchorStrategy`: pointer (40, 20) minus its offset within the
    // child at the press (10, 20).
    assert_eq!(
        context.feedback().expect("feedback").position,
        Offset::new(30., 0.)
    );
}

#[test]
fn pointer_anchor_puts_the_feedback_at_the_pointer() {
    let context = DragDropContext::<u32>::new();
    let mut tree = mount(
        Draggable::new(context.clone(), 1, card())
            .feedback(|_| card())
            .drag_anchor_strategy(DragAnchorStrategy::Pointer),
        DragTarget::new(context.clone(), card()),
    );
    drive(
        &mut tree,
        &[
            (PointerPhase::Down, 10., 20.),
            (PointerPhase::Move, 40., 20.),
        ],
    );
    assert_eq!(
        context.feedback().expect("feedback").position,
        Offset::new(40., 20.)
    );
}

#[test]
fn axis_restricts_the_feedback_movement() {
    let context = DragDropContext::<u32>::new();
    let moves = Rc::new(RefCell::new(Vec::new()));
    let mut tree = mount(
        Draggable::new(context.clone(), 1, card())
            .feedback(|_| card())
            .axis(Axis::Horizontal)
            .on_drag_update({
                let moves = moves.clone();
                move |delta| moves.borrow_mut().push(delta)
            }),
        DragTarget::new(context.clone(), card()),
    );
    drive(
        &mut tree,
        &[
            (PointerPhase::Down, 10., 20.),
            (PointerPhase::Move, 40., 20.),
            (PointerPhase::Move, 50., 60.),
        ],
    );
    // `_restrictAxis` keeps only the horizontal movement.
    assert_eq!(
        context.feedback().expect("feedback").position,
        Offset::new(40., 0.)
    );
    assert_eq!(
        moves.borrow().as_slice(),
        [Offset::new(30., 0.), Offset::new(10., 0.)]
    );
}

#[test]
fn vertical_affinity_ignores_horizontal_drags() {
    let context = DragDropContext::<u32>::new();
    let mut tree = mount(
        Draggable::new(context.clone(), 1, card()).affinity(Axis::Vertical),
        DragTarget::new(context.clone(), card()),
    );
    drive(
        &mut tree,
        &[
            (PointerPhase::Down, 10., 20.),
            (PointerPhase::Move, 60., 20.),
        ],
    );
    assert!(!context.is_dragging());
    drive(
        &mut tree,
        &[
            (PointerPhase::Up, 60., 20.),
            (PointerPhase::Down, 10., 10.),
            (PointerPhase::Move, 10., 30.),
        ],
    );
    assert!(context.is_dragging());
}

#[test]
fn zero_simultaneous_drags_disables_dragging() {
    let context = DragDropContext::<u32>::new();
    let mut tree = mount(
        Draggable::new(context.clone(), 1, card()).max_simultaneous_drags(0),
        DragTarget::new(context.clone(), card()),
    );
    drive(
        &mut tree,
        &[
            (PointerPhase::Down, 10., 20.),
            (PointerPhase::Move, 60., 20.),
        ],
    );
    assert!(!context.is_dragging());
}

#[derive(Default)]
struct Ends {
    details: RefCell<Vec<DraggableDetails>>,
    completed: Cell<usize>,
    canceled: RefCell<Vec<Offset>>,
    accepted_at: RefCell<Vec<Offset>>,
}

fn tracked(context: &DragDropContext<u32>, ends: &Rc<Ends>) -> WidgetTree {
    mount(
        Draggable::new(context.clone(), 1, card())
            .on_drag_end({
                let ends = ends.clone();
                move |details| ends.details.borrow_mut().push(details)
            })
            .on_drag_completed({
                let ends = ends.clone();
                move || ends.completed.set(ends.completed.get() + 1)
            })
            .on_draggable_canceled({
                let ends = ends.clone();
                move |_, offset| ends.canceled.borrow_mut().push(offset)
            }),
        DragTarget::new(context.clone(), card()).on_accept_with_details({
            let ends = ends.clone();
            move |_, offset| ends.accepted_at.borrow_mut().push(offset)
        }),
    )
}

#[test]
fn a_drop_on_a_target_completes_the_drag() {
    let context = DragDropContext::<u32>::new();
    let ends = Rc::new(Ends::default());
    let mut tree = tracked(&context, &ends);
    drive(
        &mut tree,
        &[
            (PointerPhase::Down, 10., 20.),
            (PointerPhase::Move, 40., 20.),
            (PointerPhase::Move, 150., 20.),
            (PointerPhase::Up, 150., 20.),
        ],
    );
    assert_eq!(ends.completed.get(), 1);
    assert!(ends.canceled.borrow().is_empty());
    let details = ends.details.borrow();
    assert_eq!(details.len(), 1);
    assert!(details[0].was_accepted);
    // The feedback's top-left: (150, 20) - (10, 20).
    assert_eq!(details[0].offset, Offset::new(140., 0.));
    assert_eq!(
        ends.accepted_at.borrow().as_slice(),
        [Offset::new(140., 0.)]
    );
}

#[test]
fn a_drop_outside_any_target_cancels_the_draggable() {
    let context = DragDropContext::<u32>::new();
    let ends = Rc::new(Ends::default());
    let mut tree = tracked(&context, &ends);
    drive(
        &mut tree,
        &[
            (PointerPhase::Down, 10., 20.),
            (PointerPhase::Move, 40., 20.),
            (PointerPhase::Move, 60., 20.),
            (PointerPhase::Up, 60., 20.),
        ],
    );
    assert_eq!(ends.completed.get(), 0);
    assert_eq!(ends.canceled.borrow().as_slice(), [Offset::new(50., 0.)]);
    assert!(!ends.details.borrow()[0].was_accepted);
}

#[test]
fn child_when_dragging_replaces_the_child_during_a_drag() {
    let context = DragDropContext::<u32>::new();
    let mut tree = WidgetTree::new();
    let root = tree
        .mount(
            Draggable::new(context.clone(), 1, card())
                .child_when_dragging(Widget::box_(Size::new(50., 40.), Color::BLACK))
                .into(),
        )
        .expect("mount");
    let constraints = Constraints::loose(Size::new(200., 200.));
    tree.layout(constraints).expect("layout");
    let size = |tree: &WidgetTree| tree.render_size(tree.render_id(root).expect("render"));
    assert_eq!(size(&tree), Some(Size::new(100., 80.)));
    drive(
        &mut tree,
        &[
            (PointerPhase::Down, 10., 20.),
            (PointerPhase::Move, 40., 20.),
        ],
    );
    tree.layout(constraints).expect("layout");
    assert_eq!(size(&tree), Some(Size::new(50., 40.)));
    drive(&mut tree, &[(PointerPhase::Up, 40., 20.)]);
    tree.layout(constraints).expect("layout");
    assert_eq!(size(&tree), Some(Size::new(100., 80.)));
}
