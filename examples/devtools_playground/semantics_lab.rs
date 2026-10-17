//! Section 7: labelled, accessible controls for the semantics tree.
use super::theme::*;
use incular::controls_prelude::{
    Button, Checkbox, GhostButton, PrimaryButton, Switch, TextField, slider,
};
use incular::prelude::*;
use incular::text::TextEditingController;

#[derive(Clone)]
pub struct SemanticsLab {
    pub checked: Signal<bool>,
    pub switched: Signal<bool>,
    pub level: Signal<f32>,
    pub presses: Signal<u32>,
    pub name: TextEditingController,
}

impl SemanticsLab {
    pub fn new() -> Self {
        Self {
            checked: Signal::new(true),
            switched: Signal::new(false),
            level: Signal::new(0.35),
            presses: Signal::new(0),
            name: TextEditingController::with_text("Ada"),
        }
    }
}

pub fn view(lab: &SemanticsLab) -> Widget {
    let presses = lab.presses.clone();
    let buttons = card(
        "Buttons",
        "Three button roles with visible labels.",
        420.,
        Column::new([
            Row::new([
                Widget::from(PrimaryButton::new("Save changes").on_click({
                    let presses = presses.clone();
                    move || presses.update(|count| *count += 1)
                })),
                hgap(10.),
                Widget::from(Button::new("Cancel").on_click({
                    let presses = presses.clone();
                    move || presses.update(|count| *count += 1)
                })),
                hgap(10.),
                Widget::from(GhostButton::new("Help")),
            ])
            .into(),
            gap(12.),
            muted(format!("Pressed {} times", presses.get())),
        ])
        .cross_axis_alignment(CrossAxisAlignment::Start),
    );

    let toggles = card(
        "Checkbox and switch",
        "State is exposed as checked / toggled.",
        420.,
        Column::new([
            Widget::from(
                Checkbox::new(lab.checked.get())
                    .label("Send me product updates")
                    .on_changed({
                        let checked = lab.checked.clone();
                        move |on| {
                            checked.set(on);
                        }
                    }),
            ),
            gap(12.),
            Row::new([
                Widget::from(Switch::new(lab.switched.get()).on_changed({
                    let switched = lab.switched.clone();
                    move |on| {
                        switched.set(on);
                    }
                }))
                .accessibility_label("Dark mode"),
                hgap(12.),
                body("Dark mode"),
            ])
            .into(),
        ])
        .cross_axis_alignment(CrossAxisAlignment::Start),
    );

    let slider_card = card(
        "Slider",
        "Numeric value, minimum and maximum are exposed.",
        420.,
        Column::new([
            Widget::from(
                slider::Root::new()
                    .value(lab.level.get())
                    .step(0.05)
                    .on_value_change({
                        let level = lab.level.clone();
                        move |value| {
                            level.set(value);
                        }
                    }),
            )
            .accessibility_label("Volume"),
            gap(8.),
            muted(format!("Volume {:.0}%", lab.level.get() * 100.)),
        ])
        .cross_axis_alignment(CrossAxisAlignment::Start),
    );

    let field = card(
        "Text field",
        "Label, value and selection are exposed.",
        420.,
        Widget::from(TextField::new(lab.name.clone()).placeholder("Your name"))
            .accessibility_label("Your name"),
    );

    page(
        "Semantics",
        "Open DevTools > Semantics to inspect roles, labels, state and actions of these controls.",
        vec![buttons, toggles, slider_card, field],
    )
}
