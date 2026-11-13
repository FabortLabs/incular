use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

use incular_animation::{
    AnimationController, AnimationSource, Curve, CurvedAnimation, DoubleAnimation,
    TrainHoppingAnimation,
};
use incular_parity_harness::{FrameHarness, Scenario};
use incular_widgets::SizedBox;

pub fn scenarios() -> Vec<Scenario> {
    vec![
        (
            "anim_compose__train_hopping__unobserved_crossing",
            train_hopping_unobserved_crossing,
        ),
        (
            "anim_compose__curved_animation__unobserved_reversal",
            curved_animation_unobserved_reversal,
        ),
    ]
}

fn curved_animation_unobserved_reversal(h: &mut FrameHarness) {
    let controller = AnimationController::new(Duration::from_millis(100));
    let curved = Rc::new(
        CurvedAnimation::new(controller.clone(), Curve::EaseIn).reverse_curve(Curve::EaseOut),
    );
    h.value("curved", move || f64::from(curved.value()));
    h.mount(|_| SizedBox::shrink().into());
    drop(controller.forward_on_next_tick());
    controller.set_value(0.5);
    drop(controller.reverse_on_next_tick());
    h.frames(1);
}

fn train_hopping_unobserved_crossing(h: &mut FrameHarness) {
    let current = AnimationController::new(Duration::from_millis(100)).with_value(0.2);
    let next = AnimationController::new(Duration::from_millis(100)).with_value(0.8);
    let switches = Rc::new(Cell::new(0));
    let train = TrainHoppingAnimation::new(current, Some(DoubleAnimation::from(next.clone())))
        .on_switched_train({
            let switches = Rc::clone(&switches);
            move || switches.set(switches.get() + 1)
        });
    h.value("switches", {
        let switches = Rc::clone(&switches);
        move || f64::from(switches.get())
    });
    h.mount(|_| SizedBox::shrink().into());
    next.set_value(0.1);
    h.frames(1);
    let _ = train.current_train();
    h.frames(1);
}
