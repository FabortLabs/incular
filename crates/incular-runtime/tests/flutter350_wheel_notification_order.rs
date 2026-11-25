//! Flutter 3.50.0 ScrollPosition.pointerScroll notification order.
use incular_config::Constraints;
use incular_core::{Color, InputEvent, Offset, PointerPhase, Size};
use incular_runtime::Runtime;
use incular_widgets::{ListView, ScrollController, ScrollNotificationType, ScrollPhysics, Widget};
use std::{cell::RefCell, rc::Rc};

fn check_wheel(physics: ScrollPhysics) {
    use ScrollNotificationType::{End, Start, Update, UserScroll};
    let controller = ScrollController::new();
    let layout = Constraints::tight(Size::new(800., 600.));
    let list = ListView::builder(40, |_| Widget::box_(Size::new(800., 50.), Color::WHITE))
        .item_extent(50.)
        .controller(controller.clone())
        .physics(physics);
    let mut runtime = Runtime::new(list.into()).unwrap();
    runtime.run_frame(layout).unwrap();
    let trace = Rc::new(RefCell::new(Vec::new()));
    let observed = trace.clone();
    let _subscription = controller.add_notification_listener(move |notification| {
        observed
            .borrow_mut()
            .push((notification.kind, notification.metrics.pixels));
        false
    });
    let _ = runtime.handle_input(InputEvent::Pointer {
        phase: PointerPhase::Move,
        position: Offset::new(400., 300.),
    });
    let mut wheel = |dy| {
        let _ = runtime.handle_input(InputEvent::Scroll {
            delta: Offset::new(0., dy),
        });
        runtime.run_frame(layout).unwrap();
        std::mem::take(&mut *trace.borrow_mut())
    };

    let edge = wheel(-40.);
    let down = wheel(60.);
    let up = wheel(-90.);
    assert_eq!(controller.offset(), 0.);
    assert_eq!(
        [edge, down, up],
        [
            vec![],
            vec![
                (UserScroll, 0.),
                (Start, 60.),
                (Update, 60.),
                (End, 60.),
                (UserScroll, 60.)
            ],
            vec![
                (UserScroll, 60.),
                (Start, 0.),
                (Update, 0.),
                (End, 0.),
                (UserScroll, 0.)
            ],
        ],
    );
}

#[test]
fn a_wheel_sample_commits_one_bracket_and_an_edge_has_no_events() {
    check_wheel(ScrollPhysics::clamping());
}

#[test]
fn bouncing_physics_still_clamp_pointer_signals_to_content() {
    check_wheel(ScrollPhysics::BOUNCING);
}
