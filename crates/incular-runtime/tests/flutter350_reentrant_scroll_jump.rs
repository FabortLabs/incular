use std::{cell::RefCell, rc::Rc};

use incular_config::Constraints;
use incular_core::Size;
use incular_runtime::Runtime;
use incular_scroll::{ScrollController, ScrollNotificationType};
use incular_widgets::{ListView, SizedBox};

#[test]
fn a_reentrant_jump_reports_its_actual_distance_from_the_prior_offset() {
    let scroll = ScrollController::new();
    let mut runtime = Runtime::new(
        ListView::new([SizedBox::new().height(2000.)])
            .controller(scroll.clone())
            .into(),
    )
    .unwrap();
    runtime
        .run_frame(Constraints::tight(Size::new(800., 600.)))
        .unwrap();
    let positions = Rc::new(RefCell::new(Vec::new()));
    let _position = scroll.add_position_listener({
        let positions = Rc::clone(&positions);
        let scroll = scroll.clone();
        move || {
            let position = scroll.offset();
            positions.borrow_mut().push(position);
            if position == 100. {
                assert!(scroll.jump_to(200.));
            }
        }
    });
    let updates = Rc::new(RefCell::new(Vec::new()));
    let _update = scroll.add_listener({
        let updates = Rc::clone(&updates);
        move |notification| {
            if notification.kind == ScrollNotificationType::Update {
                updates
                    .borrow_mut()
                    .push((notification.delta, notification.metrics.pixels));
            }
            false
        }
    });

    assert!(scroll.jump_to(100.));
    assert_eq!(scroll.offset(), 200.);
    assert_eq!(*positions.borrow(), [100., 200.]);
    assert_eq!(*updates.borrow(), [(100., 200.), (200., 200.)]);
}
