//! Design-system-neutral controls for a widget preview host.

use incular_config::MainAxisSize;
use incular_core::Transform;
use incular_widgets::{Builder, RawTooltip, Row, SizedBox, Text, TransformationController, Widget};

use crate::{Button, ButtonVariant, slider};

/// The controls and viewport share this transform. External edits refresh
/// the percentage and slider, and zoom controls replace the transform with
/// a uniform scale (or the identity on reset).
#[derive(Clone, Debug)]
pub struct PreviewZoomControls {
    controller: TransformationController,
}

impl PreviewZoomControls {
    pub const MIN_SCALE: f32 = 1.;
    pub const MAX_SCALE: f32 = 4.;
    pub const ZOOM_STEP: f32 = 0.25;

    #[must_use]
    pub const fn new(controller: TransformationController) -> Self {
        Self { controller }
    }

    #[must_use]
    pub fn scale(&self) -> f32 {
        let scale = self.controller.value().coefficients()[0] as f32;
        if scale.is_finite() {
            scale
        } else {
            Self::MIN_SCALE
        }
    }

    pub fn set_scale(&self, scale: f32) {
        if scale.is_finite() {
            self.controller.set_value(Transform::scale(
                scale.clamp(Self::MIN_SCALE, Self::MAX_SCALE),
            ));
        }
    }

    pub fn zoom_in(&self) {
        let scale = self.scale();
        if scale < Self::MAX_SCALE {
            self.set_scale(scale + Self::ZOOM_STEP);
        }
    }

    pub fn zoom_out(&self) {
        let scale = self.scale();
        if scale > Self::MIN_SCALE {
            self.set_scale(scale - Self::ZOOM_STEP);
        }
    }

    pub fn reset(&self) {
        self.controller.set_value(Transform::IDENTITY);
    }
}

fn button(
    label: &'static str,
    key: &'static str,
    enabled: bool,
    click: impl Fn() + 'static,
) -> Widget {
    let button = Button::new(label)
        .variant(ButtonVariant::Ghost)
        .enabled(enabled)
        .on_click(click);
    Widget::from(RawTooltip::new(label, button)).with_key(key)
}

impl From<PreviewZoomControls> for Widget {
    fn from(controls: PreviewZoomControls) -> Self {
        // Reuse the slider's value and focus node when the shared preview
        // transform changes, rather than replacing a focused control.
        let slider = slider::Root::new()
            .range(
                PreviewZoomControls::MIN_SCALE,
                PreviewZoomControls::MAX_SCALE,
            )
            .step(f32::EPSILON)
            .interaction_step(if cfg!(any(target_os = "ios", target_os = "macos")) {
                0.3
            } else {
                0.15
            })
            .track_extent(100.)
            .on_value_change({
                let controls = controls.clone();
                move |scale| controls.set_scale(scale)
            });
        Builder::new(move |_| {
            let scale = controls.scale();
            let label = format!("{:.0}%", (f64::from(scale) * 100.).round());
            let out = controls.clone();
            let input = controls.clone();
            let reset = controls.clone();
            Row::new([
                button(
                    "Zoom out",
                    "preview.zoom.out",
                    scale > PreviewZoomControls::MIN_SCALE,
                    move || out.zoom_out(),
                ),
                Widget::from(slider.clone().value(scale)).with_key("preview.zoom.slider"),
                button(
                    "Zoom in",
                    "preview.zoom.in",
                    scale < PreviewZoomControls::MAX_SCALE,
                    move || input.zoom_in(),
                ),
                button(
                    "Reset zoom",
                    "preview.zoom.reset",
                    scale != PreviewZoomControls::MIN_SCALE,
                    move || reset.reset(),
                ),
                Widget::from(SizedBox::new().width(42.).child(Text::new(&label)))
                    .with_key("preview.zoom.percentage"),
            ])
            .main_axis_size(MainAxisSize::Min)
            .into()
        })
        .into()
    }
}
