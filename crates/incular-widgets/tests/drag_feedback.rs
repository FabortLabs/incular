//! Framework-painted drag feedback (gestures-009). Flutter `Draggable` inserts
//! its feedback into the `Overlay` while a drag is active (`_DragAvatar._entry`)
//! and removes it when the drag ends. Incular presents the feedback through an
//! overlay portal over the draggable, so a transient surface exists exactly
//! while the drag is active.

mod common;

use common::*;
use incular_config::Constraints;
use incular_core::{Offset, Size};
use incular_widgets::*;
use std::time::{Duration, Instant};

fn mount(context: DragDropContext<String>) -> WidgetTree {
    let mut tree = WidgetTree::new();
    tree.mount(
        incular_widgets::Row::new(vec![
            Widget::from(
                Draggable::new(
                    context.clone(),
                    String::from("card"),
                    Widget::box_(Size::new(100., 80.), Color::WHITE),
                )
                .feedback(|_| Widget::box_(Size::new(100., 80.), Color::WHITE)),
            ),
            Widget::from(DragTarget::new(
                context,
                Widget::box_(Size::new(100., 80.), Color::BLACK),
            )),
        ])
        .text_direction(incular_widgets::TextDirection::Ltr)
        .into(),
    )
    .unwrap();
    tree.layout(Constraints::tight(Size::new(200., 80.)))
        .expect("layout");
    tree
}

fn pump(tree: &mut WidgetTree, now: Instant) {
    tree.update_compositor(now).expect("frame");
    tree.layout(Constraints::tight(Size::new(200., 80.)))
        .expect("layout");
}

#[test]
fn feedback_is_presented_only_while_the_drag_is_active() {
    let context = DragDropContext::new();
    let mut tree = mount(context.clone());
    let base = Instant::now();
    pump(&mut tree, base);
    assert!(
        tree.transient_surfaces().is_empty(),
        "no feedback before the drag starts"
    );

    for (millis, phase, x) in [
        (0, incular_core::PointerPhase::Down, 10.),
        (10, incular_core::PointerPhase::Move, 35.),
        (20, incular_core::PointerPhase::Move, 60.),
    ] {
        let _ = tree.dispatch_gesture(incular_gestures::PointerEvent {
            pointer: 50,
            position: Offset::new(x, 20.),
            phase,
            time: base + Duration::from_millis(millis),
        });
    }
    assert!(context.is_dragging());
    pump(&mut tree, base + Duration::from_millis(30));
    assert_eq!(
        tree.transient_surfaces().len(),
        1,
        "the feedback is presented during the drag"
    );

    let _ = tree.dispatch_gesture(incular_gestures::PointerEvent {
        pointer: 50,
        position: Offset::new(60., 20.),
        phase: incular_core::PointerPhase::Up,
        time: base + Duration::from_millis(40),
    });
    pump(&mut tree, base + Duration::from_millis(50));
    assert!(!context.is_dragging());
    assert!(
        tree.transient_surfaces().is_empty(),
        "the feedback is removed when the drag ends"
    );
}
