//! Section 4: DevTools-visible, editable signals.
use super::theme::*;
use incular::controls_prelude::{Button, PrimaryButton, Switch};
use incular::prelude::*;
use std::fmt::Debug;

/// Registers `signal` with the DevTools Signals tab when the `devtools`
/// feature is enabled; otherwise returns it unchanged.
pub fn named<T: Debug + PartialEq + 'static>(signal: Signal<T>, name: &str) -> Signal<T> {
    #[cfg(feature = "devtools")]
    {
        signal.devtools(name).devtools_editable()
    }
    #[cfg(not(feature = "devtools"))]
    {
        let _ = name;
        signal
    }
}

#[derive(Clone)]
pub struct Signals {
    pub counter: Signal<i64>,
    pub label: Signal<String>,
    pub enabled: Signal<bool>,
    pub ratio: Signal<f64>,
}

impl Signals {
    pub fn new() -> Self {
        Self {
            counter: named(Signal::new(0_i64), "playground.counter"),
            label: named(Signal::new("hello".to_owned()), "playground.label"),
            enabled: named(Signal::new(true), "playground.enabled"),
            ratio: named(Signal::new(0.5_f64), "playground.ratio"),
        }
    }
}

pub fn view(signals: &Signals) -> Widget {
    let counter = signals.counter.clone();
    let label = signals.label.clone();
    let enabled = signals.enabled.clone();
    let ratio = signals.ratio.clone();

    let counter_card = card(
        "Counter signal",
        "playground.counter (i64): editable from the DevTools Signals tab.",
        420.,
        Column::new([
            text(
                format!("count = {}", counter.get()),
                30.,
                FontWeight::W800,
                VIOLET,
            ),
            gap(12.),
            Row::new([
                Widget::from(PrimaryButton::new("Increment").on_click({
                    let counter = counter.clone();
                    move || counter.update(|value| *value += 1)
                })),
                hgap(10.),
                Widget::from(Button::new("Decrement").on_click({
                    let counter = counter.clone();
                    move || counter.update(|value| *value -= 1)
                })),
                hgap(10.),
                Widget::from(Button::new("Reset").on_click({
                    let counter = counter.clone();
                    move || {
                        counter.set(0);
                    }
                })),
            ])
            .into(),
        ])
        .cross_axis_alignment(CrossAxisAlignment::Start),
    );

    let text_card = card(
        "Text signal",
        "playground.label (String): edit it remotely and watch this card.",
        420.,
        Column::new([
            text(label.get(), 22., FontWeight::W700, INK),
            gap(12.),
            Widget::from(Button::new("Append \"!\"").on_click({
                let label = label.clone();
                move || label.update(|value| value.push('!'))
            })),
        ])
        .cross_axis_alignment(CrossAxisAlignment::Start),
    );

    let ratio_value = ratio.get();
    let flag_card = card(
        "Bool and float signals",
        "playground.enabled (bool), playground.ratio (f64).",
        420.,
        Column::new([
            Row::new([
                Widget::from(Switch::new(enabled.get()).on_changed({
                    let enabled = enabled.clone();
                    move |on| {
                        enabled.set(on);
                    }
                }))
                .accessibility_label("Enabled signal"),
                hgap(12.),
                body(format!("enabled = {}", enabled.get())),
            ])
            .into(),
            gap(14.),
            body(format!("ratio = {ratio_value:.2}")),
            gap(6.),
            Container::new()
                .width(260.)
                .height(14.)
                .radius(7.)
                .color(LINE)
                .alignment(Alignment::CENTER_LEFT)
                .child(
                    Container::new()
                        .width(260. * ratio_value.clamp(0., 1.) as f32)
                        .height(14.)
                        .radius(7.)
                        .color(if enabled.get() { TEAL } else { MUTED }),
                )
                .into(),
            gap(10.),
            Widget::from(Button::new("Ratio +0.1").on_click({
                let ratio = ratio.clone();
                move || ratio.update(|value| *value = (*value + 0.1).min(1.))
            })),
        ])
        .cross_axis_alignment(CrossAxisAlignment::Start),
    );

    page(
        "Signals",
        "Named signals appear in the DevTools Signals tab with subscribers and edit support.",
        vec![counter_card, text_card, flag_card],
    )
}
