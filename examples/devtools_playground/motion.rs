//! Section 1: continuously animating widgets, so frames stream constantly.
use super::theme::*;
use incular::controls_prelude::Progress;
use incular::prelude::*;
use std::time::Duration;

#[derive(Clone)]
pub struct Motion {
    slide: AnimationController,
    spin: AnimationController,
}

impl Motion {
    pub fn new() -> Self {
        Self {
            slide: AnimationController::new(Duration::from_millis(2400)),
            spin: AnimationController::new(Duration::from_millis(1100)),
        }
    }
}

const TRACK: f32 = 230.;
const CARD: f32 = 130.;

pub fn view(motion: &Motion) -> Widget {
    let spinner = ticking(&motion.spin, false, |value| {
        Transform::rotation(
            value * std::f32::consts::TAU,
            Container::new()
                .width(64.)
                .height(64.)
                .radius(14.)
                .color(VIOLET)
                .alignment(Alignment::TOP_CENTER)
                .padding(EdgeInsets::all(8.))
                .child(swatch(14., 14., SURFACE)),
        )
        .into()
    });
    let spin_value = motion.spin.clone();
    let spinner_card = card(
        "Spinner",
        "A rotating box and an indeterminate progress bar, always running.",
        420.,
        Column::new([
            Container::new()
                .height(90.)
                .alignment(Alignment::CENTER_LEFT)
                .child(spinner)
                .into(),
            gap(10.),
            Progress::new().indeterminate(true).width(300.).into(),
            gap(12.),
            ticking(&spin_value, false, move |value| {
                muted(format!("rotation: {value:.3} turn"))
            }),
        ])
        .cross_axis_alignment(CrossAxisAlignment::Start),
    );

    let slide = motion.slide.clone();
    let slider = ticking(&motion.slide, true, move |value| {
        let eased = value * value * (3. - 2. * value);
        Container::new()
            .width(TRACK + CARD)
            .height(96.)
            .color(VIOLET_SOFT)
            .radius(12.)
            .alignment(Alignment::CENTER_LEFT)
            .child(Transform::translation(
                Offset::new(eased * TRACK, 0.),
                Container::new()
                    .width(CARD)
                    .height(64.)
                    .radius(12.)
                    .color(TEAL)
                    .alignment(Alignment::CENTER)
                    .child(text("sliding card", 14., FontWeight::W700, SURFACE)),
            ))
            .into()
    });
    let slide_card = card(
        "Sliding card",
        "Ping-pongs forever. Change DevTools animation speed to slow it down or speed it up.",
        420.,
        Column::new([
            slider,
            gap(12.),
            ticking(&motion.slide, true, move |_| {
                muted(format!(
                    "controller value: {:.4}  (status {:?})",
                    slide.value(),
                    slide.status()
                ))
            }),
        ])
        .cross_axis_alignment(CrossAxisAlignment::Start),
    );
    page(
        "Live motion",
        "Frames stream continuously here; use the Performance view and the animation speed control.",
        vec![spinner_card, slide_card],
    )
}
