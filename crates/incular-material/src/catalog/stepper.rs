//! Flutter's `Stepper` and `Step` (`material/stepper.dart`).

use std::rc::Rc;

use incular_config::{CrossAxisAlignment, EdgeInsets, MainAxisSize};
use incular_widgets::{
    BoxDecoration, BoxShape, Center, Column, Container, Expanded, GestureDetector, IntrinsicHeight,
    Row, SizedBox, Text, Widget,
};

use super::theme_of;
use crate::material_theme::helpers::alpha;
use crate::{FilledButton, StepState, StepperType, TextButton};

/// Flutter's `Step`.
#[derive(Clone)]
pub struct Step {
    pub title: Widget,
    pub subtitle: Option<Widget>,
    pub content: Widget,
    pub state: StepState,
    pub is_active: bool,
    pub label: Option<Widget>,
}

impl Step {
    #[must_use]
    pub fn new(title: impl Into<Widget>, content: impl Into<Widget>) -> Self {
        Self {
            title: title.into(),
            subtitle: None,
            content: content.into(),
            state: StepState::Indexed,
            is_active: false,
            label: None,
        }
    }

    #[must_use]
    pub fn subtitle(mut self, subtitle: impl Into<Widget>) -> Self {
        self.subtitle = Some(subtitle.into());
        self
    }

    #[must_use]
    pub fn state(mut self, state: StepState) -> Self {
        self.state = state;
        self
    }

    #[must_use]
    pub fn is_active(mut self, active: bool) -> Self {
        self.is_active = active;
        self
    }

    #[must_use]
    pub fn label(mut self, label: impl Into<Widget>) -> Self {
        self.label = Some(label.into());
        self
    }
}

type IndexCallback = Rc<dyn Fn(usize)>;
type Callback = Rc<dyn Fn()>;

/// Flutter's `Stepper`: numbered steps, the current one expanded with
/// continue and cancel controls.
#[derive(Clone)]
pub struct Stepper {
    steps: Vec<Step>,
    current_step: usize,
    stepper_type: StepperType,
    on_step_tapped: Option<IndexCallback>,
    on_step_continue: Option<Callback>,
    on_step_cancel: Option<Callback>,
    continue_label: String,
    cancel_label: String,
}

/// `_kStepSize`, `_kTriangleHeight` and the vertical layout margins.
const STEP_SIZE: f32 = 24.;

impl Stepper {
    #[must_use]
    pub fn new(steps: impl IntoIterator<Item = Step>) -> Self {
        Self {
            steps: steps.into_iter().collect(),
            current_step: 0,
            stepper_type: StepperType::Vertical,
            on_step_tapped: None,
            on_step_continue: None,
            on_step_cancel: None,
            continue_label: "Continue".to_owned(),
            cancel_label: "Cancel".to_owned(),
        }
    }

    #[must_use]
    pub fn current_step(mut self, index: usize) -> Self {
        self.current_step = index;
        self
    }

    #[must_use]
    pub fn stepper_type(mut self, stepper_type: StepperType) -> Self {
        self.stepper_type = stepper_type;
        self
    }

    #[must_use]
    pub fn on_step_tapped(mut self, callback: impl Fn(usize) + 'static) -> Self {
        self.on_step_tapped = Some(Rc::new(callback));
        self
    }

    #[must_use]
    pub fn on_step_continue(mut self, callback: impl Fn() + 'static) -> Self {
        self.on_step_continue = Some(Rc::new(callback));
        self
    }

    #[must_use]
    pub fn on_step_cancel(mut self, callback: impl Fn() + 'static) -> Self {
        self.on_step_cancel = Some(Rc::new(callback));
        self
    }
}

impl From<Stepper> for Widget {
    fn from(stepper: Stepper) -> Self {
        let stepper = Rc::new(stepper);
        incular_widgets::Builder::new(move |context| {
            let theme = theme_of(context);
            match stepper.stepper_type {
                StepperType::Vertical => vertical(&stepper, &theme),
                StepperType::Horizontal => horizontal(&stepper, &theme),
            }
        })
        .into()
    }
}

/// `_buildCircle`: the step number, a check when complete, an edit glyph
/// while editing, `!` on error.
fn step_icon(step: &Step, index: usize, theme: &crate::ThemeData) -> Widget {
    let scheme = &theme.core().color_scheme;
    let text = &theme.core().text_theme;
    let (glyph, background, foreground) = match step.state {
        StepState::Error => ("!".to_owned(), scheme.error, scheme.on_error),
        StepState::Complete => (
            "✓".to_owned(),
            circle_color(step, scheme),
            scheme.on_primary,
        ),
        StepState::Editing => (
            "✎".to_owned(),
            circle_color(step, scheme),
            scheme.on_primary,
        ),
        StepState::Indexed | StepState::Disabled => (
            (index + 1).to_string(),
            circle_color(step, scheme),
            scheme.on_primary,
        ),
    };
    Container::new()
        .width(STEP_SIZE)
        .height(STEP_SIZE)
        .decoration(
            BoxDecoration::new()
                .shape(BoxShape::Circle)
                .color(background),
        )
        .child(Center::new(
            Text::new(glyph)
                .style(text.body_small.clone())
                .color(foreground),
        ))
        .into()
}

/// `_circleColor`: primary for active steps, `onSurface` at 38% otherwise.
fn circle_color(step: &Step, scheme: &crate::ColorScheme) -> incular_core::Color {
    if step.is_active {
        scheme.primary
    } else {
        alpha(scheme.on_surface, 0.38)
    }
}

fn header_text(step: &Step, theme: &crate::ThemeData) -> Widget {
    let scheme = &theme.core().color_scheme;
    let text = &theme.core().text_theme;
    // `_titleStyle`/`_subtitleStyle`.
    let color = match step.state {
        StepState::Error => scheme.error,
        StepState::Disabled => alpha(scheme.on_surface, 0.38),
        _ => scheme.on_surface,
    };
    let mut lines = vec![Widget::from(incular_widgets::DefaultTextStyle::new(
        text.body_large.clone().color(color),
        step.title.clone(),
    ))];
    if let Some(subtitle) = &step.subtitle {
        lines.push(
            incular_widgets::DefaultTextStyle::new(
                text.body_small.clone().color(alpha(color, 0.75)),
                subtitle.clone(),
            )
            .into(),
        );
    }
    Column::new(lines)
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .into()
}

fn tappable(stepper: &Stepper, index: usize, child: Widget) -> Widget {
    let enabled = stepper.steps[index].state != StepState::Disabled;
    match stepper.on_step_tapped.clone() {
        Some(callback) if enabled => GestureDetector::new(child)
            .on_tap(move || callback(index))
            .into(),
        _ => child,
    }
}

/// `_buildVerticalControls`: a filled Continue and a text Cancel button.
fn controls(stepper: &Stepper) -> Widget {
    let mut continue_button = FilledButton::new(stepper.continue_label.clone())
        .enabled(stepper.on_step_continue.is_some());
    if let Some(callback) = stepper.on_step_continue.clone() {
        continue_button = continue_button.on_pressed(move || callback());
    }
    let mut cancel =
        TextButton::new(stepper.cancel_label.clone()).enabled(stepper.on_step_cancel.is_some());
    if let Some(callback) = stepper.on_step_cancel.clone() {
        cancel = cancel.on_pressed(move || callback());
    }
    Container::new()
        .margin(EdgeInsets::only(0., 16., 0., 0.))
        .child(
            SizedBox::new().height(48.).child(
                Row::new([
                    Widget::from(continue_button),
                    SizedBox::new().width(8.).into(),
                    Widget::from(cancel),
                ])
                .main_axis_size(MainAxisSize::Min),
            ),
        )
        .into()
}

fn vertical(stepper: &Rc<Stepper>, theme: &crate::ThemeData) -> Widget {
    let scheme = &theme.core().color_scheme;
    let line = alpha(scheme.on_surface, 0.38);
    let count = stepper.steps.len();
    let mut rows = Vec::new();
    for (index, step) in stepper.steps.iter().enumerate() {
        // `_buildVerticalHeader`: 24px margins, a 24px circle, 12px gap.
        let header = Container::new()
            .margin(EdgeInsets::symmetric(24., 0.))
            .child(
                Row::new([
                    step_icon(step, index, theme),
                    SizedBox::new().width(12.).into(),
                    Expanded::new(header_text(step, theme)).into(),
                ])
                .cross_axis_alignment(CrossAxisAlignment::Center),
            );
        rows.push(tappable(
            stepper,
            index,
            SizedBox::new().height(72.).child(header).into(),
        ));
        // `_buildVerticalBody`: the connector line at 36px and the content
        // indented to 60px; only the current step shows its content.
        let current = index == stepper.current_step;
        let mut body: Vec<Widget> = Vec::new();
        if current {
            body.push(step.content.clone());
            body.push(controls(stepper));
        }
        let connector: Widget = if index + 1 < count {
            Container::new().width(1.).color(line).into()
        } else {
            SizedBox::new().width(1.).into()
        };
        rows.push(
            IntrinsicHeight::new(
                Row::new([
                    SizedBox::new().width(36.).into(),
                    connector,
                    Expanded::new(
                        Container::new()
                            .margin(EdgeInsets::only(
                                23.,
                                0.,
                                24.,
                                if current { 24. } else { 0. },
                            ))
                            .child(
                                Column::new(body)
                                    .main_axis_size(MainAxisSize::Min)
                                    .cross_axis_alignment(CrossAxisAlignment::Start),
                            ),
                    )
                    .into(),
                ])
                .cross_axis_alignment(CrossAxisAlignment::Stretch),
            )
            .into(),
        );
    }
    Column::new(rows)
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .into()
}

fn horizontal(stepper: &Rc<Stepper>, theme: &crate::ThemeData) -> Widget {
    let scheme = &theme.core().color_scheme;
    let count = stepper.steps.len();
    let mut header: Vec<Widget> = Vec::new();
    for (index, step) in stepper.steps.iter().enumerate() {
        let item = Row::new([
            step_icon(step, index, theme),
            SizedBox::new().width(12.).into(),
            header_text(step, theme),
        ])
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(CrossAxisAlignment::Center);
        header.push(tappable(
            stepper,
            index,
            SizedBox::new().height(72.).child(Center::new(item)).into(),
        ));
        if index + 1 < count {
            header.push(
                Expanded::new(
                    Container::new()
                        .margin(EdgeInsets::symmetric(8., 0.))
                        .height(1.)
                        .color(scheme.outline_variant),
                )
                .into(),
            );
        }
    }
    let current = stepper.steps.get(stepper.current_step);
    let mut column = vec![Widget::from(
        crate::Material::new(
            Container::new()
                .margin(EdgeInsets::symmetric(24., 0.))
                .child(Row::new(header).cross_axis_alignment(CrossAxisAlignment::Center)),
        )
        .elevation(2.),
    )];
    if let Some(step) = current {
        column.push(
            Container::new()
                .padding(EdgeInsets::all(24.))
                .child(
                    Column::new([step.content.clone(), controls(stepper)])
                        .main_axis_size(MainAxisSize::Min)
                        .cross_axis_alignment(CrossAxisAlignment::Start),
                )
                .into(),
        );
    }
    Column::new(column)
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .into()
}
