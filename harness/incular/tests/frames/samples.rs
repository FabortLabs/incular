//! Sample frame scenarios. Each one has a twin with the same name in
//! harness/flutter/test/frames/samples_test.dart; keep them identical.

use std::time::Duration;

use incular_core::{Color, Offset};
use incular_parity_harness::{FrameHarness, Scenario, probe};
use incular_widgets::{
    AnimatedContainer, AnimatedOpacity, AnimatedPositioned, AnimatedRotation, Center, ColoredBox,
    Curves, ListView, ScrollController, SizedBox, Stack, Widget,
};

const BLUE: Color = Color::rgba(0x21, 0x96, 0xF3, 0xFF);
const ORANGE: Color = Color::rgba(0xFF, 0x57, 0x22, 0xFF);
const GREEN: Color = Color::rgba(0x4C, 0xAF, 0x50, 0xFF);

pub fn scenarios() -> Vec<Scenario> {
    vec![
        ("sample_animated_container", animated_container),
        ("sample_animated_opacity", animated_opacity),
        ("sample_animated_positioned", animated_positioned),
        ("sample_animated_rotation", animated_rotation),
        ("sample_list_fling", list_fling),
    ]
}

/// A Flutter `ColoredBox` without a child.
fn colored(color: Color) -> Widget {
    ColoredBox::new(color, SizedBox::shrink()).into()
}

fn animated_container(h: &mut FrameHarness) {
    h.mount(|phase| {
        Center::new(probe(
            "box",
            AnimatedContainer::new(Duration::from_millis(300))
                .curve(Curves::EASE_IN_OUT)
                .width(if phase == 0 { 100. } else { 200. })
                .height(if phase == 0 { 100. } else { 60. })
                .color(if phase == 0 { BLUE } else { ORANGE }),
        ))
        .into()
    });
    h.frames(2);
    h.set_phase(1);
    h.frames(25);
}

fn animated_opacity(h: &mut FrameHarness) {
    h.mount(|phase| {
        Center::new(AnimatedOpacity::new(
            if phase == 0 { 1. } else { 0.2 },
            Duration::from_millis(200),
            probe(
                "box",
                SizedBox::new().width(80.).height(80.).child(colored(GREEN)),
            ),
        ))
        .into()
    });
    h.frames(2);
    h.set_phase(1);
    h.frames(18);
}

fn animated_positioned(h: &mut FrameHarness) {
    h.mount(|phase| {
        Stack::new([Widget::from(
            AnimatedPositioned::new(Duration::from_millis(400), probe("box", colored(ORANGE)))
                .curve(Curves::FAST_OUT_SLOW_IN)
                .left(if phase == 0 { 0. } else { 300. })
                .top(50.)
                .width(100.)
                .height(100.),
        )])
        .into()
    });
    h.frames(2);
    h.set_phase(1);
    h.frames(30);
}

fn animated_rotation(h: &mut FrameHarness) {
    h.mount(|phase| {
        Center::new(AnimatedRotation::new(
            if phase == 0 { 0. } else { 0.25 },
            Duration::from_millis(300),
            probe(
                "box",
                SizedBox::new().width(100.).height(50.).child(colored(BLUE)),
            ),
        ))
        .into()
    });
    h.frames(2);
    h.set_phase(1);
    h.frames(25);
}

fn list_fling(h: &mut FrameHarness) {
    let controller = ScrollController::new();
    let read = controller.clone();
    h.value("offset", move || f64::from(read.offset()));
    h.mount(move |_| {
        ListView::builder(200, |index| {
            let row = colored(if index % 2 == 0 { BLUE } else { GREEN });
            if index == 0 || index == 20 {
                probe(&format!("row{index}"), row)
            } else {
                row
            }
        })
        .item_extent(50.)
        .controller(controller.clone())
        .into()
    });
    h.frames(1);
    h.down(Offset::new(400., 400.));
    for step in 1..=6 {
        h.frames(1);
        h.move_to(Offset::new(400., 400. - 40. * step as f32));
    }
    h.frames(1);
    h.up();
    h.frames(90);
}
