use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

use incular_animation::{AnimationController, DoubleAnimation, TrainHoppingAnimation};
use incular_parity_harness::{FrameHarness, Scenario};
use incular_widgets::SizedBox;

pub fn scenarios() -> Vec<Scenario> {
    vec![(
        "anim_compose__train_hopping__unobserved_crossing",
        train_hopping_unobserved_crossing,
    )]
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
