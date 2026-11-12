use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

use incular_animation::{
    AnimationController, AnimationStatus, RepeatOptions, Simulation, Ticker,
    add_post_frame_callback, cancel_frame_callback, schedule_frame_callback,
};
use incular_parity_harness::{FrameHarness, Scenario};
use incular_widgets::{SizedBox, Widget};

fn status_code(status: AnimationStatus) -> f64 {
    match status {
        AnimationStatus::Dismissed => 0.,
        AnimationStatus::Forward => 1.,
        AnimationStatus::Reverse => 2.,
        AnimationStatus::Completed => 3.,
    }
}

#[derive(Clone, Copy)]
struct StepSimulation {
    start: f32,
    distance: f32,
    duration: Duration,
}

impl Simulation for StepSimulation {
    fn position(&self, elapsed: Duration) -> f32 {
        let total = self.duration.as_secs_f32();
        self.start + self.distance * (elapsed.as_secs_f32() / total).clamp(0., 1.)
    }

    fn velocity(&self, elapsed: Duration) -> f32 {
        if elapsed >= self.duration {
            0.
        } else {
            self.distance / self.duration.as_secs_f32()
        }
    }

    fn is_done(&self, elapsed: Duration) -> bool {
        elapsed >= self.duration
    }
}

pub fn scenarios() -> Vec<Scenario> {
    vec![
        (
            "anim_core__controller_listener_notifications",
            controller_listener_notifications,
        ),
        ("anim_core__animate_to_repeat", animate_to_repeat),
        ("anim_core__animate_with_fling", animate_with_fling),
        (
            "anim_core__scheduler_callbacks_time_dilation",
            scheduler_callbacks_time_dilation,
        ),
    ]
}

fn controller_listener_notifications(h: &mut FrameHarness) {
    let controller = AnimationController::with_bounds(Duration::from_millis(160), 0.2, 0.8)
        .with_reverse_duration(Duration::from_millis(80))
        .with_value(0.25)
        .with_vsync();
    let value_notifications = Rc::new(Cell::new(0_u32));
    let status_notifications = Rc::new(Cell::new(0_u32));
    let forward_outcome = Rc::new(Cell::new(0_i32));
    controller.add_listener({
        let value_notifications = Rc::clone(&value_notifications);
        move |_| value_notifications.set(value_notifications.get() + 1)
    });
    controller.add_status_listener({
        let status_notifications = Rc::clone(&status_notifications);
        move |_| status_notifications.set(status_notifications.get() + 1)
    });
    {
        let controller = controller.clone();
        h.value("value", move || f64::from(controller.value()));
    }
    {
        let controller = controller.clone();
        h.value("status", move || status_code(controller.status()));
    }
    {
        let controller = controller.clone();
        h.value("is_animating", move || {
            if controller.is_animating() { 1. } else { 0. }
        });
    }
    h.value("value_notifications", {
        let value_notifications = Rc::clone(&value_notifications);
        move || f64::from(value_notifications.get())
    });
    h.value("status_notifications", {
        let status_notifications = Rc::clone(&status_notifications);
        move || f64::from(status_notifications.get())
    });
    h.value("forward_outcome", {
        let forward_outcome = Rc::clone(&forward_outcome);
        move || f64::from(forward_outcome.get())
    });

    h.mount(|_| SizedBox::shrink().into());
    let forward = controller.forward_on_next_tick();
    {
        let outcome = Rc::clone(&forward_outcome);
        let future = forward.clone();
        forward.when_complete_or_cancel(move || {
            outcome.set(if future.is_canceled() { -1 } else { 1 });
        });
    }
    h.frames(3);
    controller.stop();
    h.frames(1);
    controller.set_value(controller.value());
    h.frames(1);
    controller.set_value(0.75);
    controller.reverse_on_next_tick();
    h.frames(7);
    controller.reset();
    h.frames(1);
    controller.animate_to_on_next_tick_curved(
        0.5,
        Some(Duration::from_millis(80)),
        incular_animation::Curve::Linear,
    );
    h.frames(7);
    controller.set_value(1.5);
    h.frames(1);
}

fn animate_to_repeat(h: &mut FrameHarness) {
    let controller = AnimationController::new(Duration::from_millis(160))
        .with_value(0.75)
        .with_vsync();
    let animate_outcome = Rc::new(Cell::new(0_i32));
    let repeat_outcome = Rc::new(Cell::new(0_i32));
    {
        let controller = controller.clone();
        h.value("value", move || f64::from(controller.value()));
    }
    {
        let controller = controller.clone();
        h.value("status", move || status_code(controller.status()));
    }
    {
        let controller = controller.clone();
        h.value("is_animating", move || {
            if controller.is_animating() { 1. } else { 0. }
        });
    }
    h.value("animate_outcome", {
        let animate_outcome = Rc::clone(&animate_outcome);
        move || f64::from(animate_outcome.get())
    });
    h.value("repeat_outcome", {
        let repeat_outcome = Rc::clone(&repeat_outcome);
        move || f64::from(repeat_outcome.get())
    });

    h.mount(|_| SizedBox::shrink().into());
    let animate = controller.animate_to_on_next_tick(0.25);
    {
        let outcome = Rc::clone(&animate_outcome);
        let future = animate.clone();
        animate.when_complete_or_cancel(move || {
            outcome.set(if future.is_canceled() { -1 } else { 1 });
        });
    }
    h.frames(7);

    let repeat = controller.repeat_on_next_tick(
        RepeatOptions::default()
            .min(0.2)
            .max(0.8)
            .reverse(true)
            .period(Duration::from_millis(64))
            .count(3),
    );
    {
        let outcome = Rc::clone(&repeat_outcome);
        let future = repeat.clone();
        repeat.when_complete_or_cancel(move || {
            outcome.set(if future.is_canceled() { -1 } else { 1 });
        });
    }
    h.frames(14);
}

fn animate_with_fling(h: &mut FrameHarness) {
    let controller = AnimationController::new(Duration::from_millis(160))
        .with_value(0.2)
        .with_vsync();
    let simulation_outcome = Rc::new(Cell::new(0_i32));
    let fling_outcome = Rc::new(Cell::new(0_i32));
    {
        let controller = controller.clone();
        h.value("value", move || f64::from(controller.value()));
    }
    {
        let controller = controller.clone();
        h.value("status", move || status_code(controller.status()));
    }
    {
        let controller = controller.clone();
        h.value("is_animating", move || {
            if controller.is_animating() { 1. } else { 0. }
        });
    }
    h.value("simulation_outcome", {
        let simulation_outcome = Rc::clone(&simulation_outcome);
        move || f64::from(simulation_outcome.get())
    });
    h.value("fling_outcome", {
        let fling_outcome = Rc::clone(&fling_outcome);
        move || f64::from(fling_outcome.get())
    });

    h.mount(|_| SizedBox::shrink().into());
    let simulation = controller.animate_with(StepSimulation {
        start: 0.2,
        distance: 0.4,
        duration: Duration::from_millis(64),
    });
    {
        let outcome = Rc::clone(&simulation_outcome);
        let future = simulation.clone();
        simulation.when_complete_or_cancel(move || {
            outcome.set(if future.is_canceled() { -1 } else { 1 });
        });
    }
    h.frames(5);

    controller.set_value(0.35);
    let fling = controller.fling_on_next_tick(1.0);
    {
        let outcome = Rc::clone(&fling_outcome);
        let future = fling.clone();
        fling.when_complete_or_cancel(move || {
            outcome.set(if future.is_canceled() { -1 } else { 1 });
        });
    }
    h.frames(100);
}

fn scheduler_callbacks_time_dilation(h: &mut FrameHarness) {
    let controller = AnimationController::new(Duration::from_millis(320));
    let ticker_elapsed_us = Rc::new(Cell::new(0.0_f64));
    let ticker = Ticker::new({
        let ticker_elapsed_us = Rc::clone(&ticker_elapsed_us);
        move |elapsed| ticker_elapsed_us.set(elapsed.as_secs_f64() * 1_000_000.0)
    });
    let begin_calls = Rc::new(Cell::new(0_u32));
    let canceled_calls = Rc::new(Cell::new(0_u32));
    let post_calls = Rc::new(Cell::new(0_u32));
    {
        let controller = controller.clone();
        h.value("value", move || f64::from(controller.value()));
    }
    {
        let controller = controller.clone();
        h.value("status", move || status_code(controller.status()));
    }
    h.value("ticker_elapsed_us", {
        let ticker_elapsed_us = Rc::clone(&ticker_elapsed_us);
        move || ticker_elapsed_us.get()
    });
    h.value("begin_calls", {
        let begin_calls = Rc::clone(&begin_calls);
        move || f64::from(begin_calls.get())
    });
    h.value("canceled_calls", {
        let canceled_calls = Rc::clone(&canceled_calls);
        move || f64::from(canceled_calls.get())
    });
    h.value("post_calls", {
        let post_calls = Rc::clone(&post_calls);
        move || f64::from(post_calls.get())
    });

    let root_controller = controller.clone();
    h.mount(move |_| Widget::animation_ticker(root_controller.clone(), SizedBox::shrink()));
    let begin = Rc::clone(&begin_calls);
    schedule_frame_callback(move |_| begin.set(begin.get() + 1));
    let canceled = Rc::clone(&canceled_calls);
    let canceled_id = schedule_frame_callback(move |_| canceled.set(canceled.get() + 1));
    cancel_frame_callback(canceled_id);
    let post = Rc::clone(&post_calls);
    add_post_frame_callback(move |_| post.set(post.get() + 1));
    h.runtime().set_animation_time_scale(0.5);
    ticker.start();
    controller.forward_on_next_tick();
    h.frames(8);
    controller.stop();
    ticker.stop();
    h.frames(1);
    h.runtime().set_animation_time_scale(1.0);
    h.frames(1);
}
