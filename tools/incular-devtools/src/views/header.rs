//! Header widgets that are visible from every view: live FPS, the picker
//! indicator and the Overlays menu button.

use super::shared::{danger, on_accent, state_surface, success, text_primary, warning};
use super::{control, control_active, gap, primary, text_muted, ui_text};
use crate::performance::LiveFps;
use incular::material::{RawMaterialButton, Tooltip};
use incular::prelude::*;

/// Frame time that fits a 60 Hz display, used as the sparkline's floor.
const BUDGET_US: u32 = 16_667;
const SPARK_WIDTH: f32 = 60.;
const SPARK_HEIGHT: f32 = 14.;

/// Colour for a live FPS reading: green near display rate, amber when the
/// app is struggling, red when it is slow, muted when it is not redrawing.
pub(crate) fn fps_color(live: LiveFps) -> Color {
    match live.fps {
        _ if live.idle => text_muted(),
        55.. => success(),
        30..=54 => warning(),
        _ => danger(),
    }
}

fn sparkline(samples: &[u32]) -> Widget {
    let mut canvas = Canvas::default();
    let ceiling = samples.iter().copied().max().unwrap_or(0).max(BUDGET_US) as f32;
    // One pixel per frame, newest on the right; short histories hug the right
    // edge so the graph reads as scrolling in from there.
    for (index, micros) in samples.iter().rev().take(SPARK_WIDTH as usize).enumerate() {
        let height = (*micros as f32 / ceiling * SPARK_HEIGHT).clamp(1., SPARK_HEIGHT);
        canvas.rect(
            incular::core::Rect::from_origin_size(
                Offset::new(SPARK_WIDTH - 1. - index as f32, SPARK_HEIGHT - height),
                Size::new(1., height),
            ),
            if *micros > BUDGET_US {
                danger()
            } else {
                success()
            },
        );
    }
    CustomPaint::new(Size::new(SPARK_WIDTH, SPARK_HEIGHT), canvas.finish()).into()
}

/// Live FPS pill with a sparkline of recent frame CPU times. Clicking opens
/// the Performance view.
pub(crate) fn fps_pill(
    live: LiveFps,
    cpu_micros: &[u32],
    connected: bool,
    open_performance: impl Fn() + 'static,
) -> Widget {
    let label = if !connected {
        "offline".to_owned()
    } else if live.idle {
        "idle".to_owned()
    } else {
        format!("{} FPS", live.fps)
    };
    let tip = if !connected {
        "Not connected to an app."
    } else if live.idle {
        "No frame in the last second: the app is not redrawing. DevTools counts the frames the \
         app presents, so a static UI reads idle. Click to open Performance."
    } else {
        "Frames the app presented in the last second, with the CPU time of the last 60 frames. \
         Click to open Performance."
    };
    let color = if connected {
        fps_color(live)
    } else {
        text_muted()
    };
    let content = Row::new([
        Widget::from(
            Container::new()
                .width(8.)
                .height(8.)
                .color(color)
                .radius(4.),
        ),
        gap(7., 1.),
        ui_text(
            label.clone(),
            12.,
            if connected {
                text_primary()
            } else {
                text_muted()
            },
        ),
        gap(8., 1.),
        sparkline(cpu_micros),
    ])
    .main_axis_size(MainAxisSize::Min)
    .cross_axis_alignment(CrossAxisAlignment::Center);
    let button = RawMaterialButton::new(format!("Live frame rate: {label}"))
        .size(Size::new(0., 30.))
        .padding(EdgeInsets::symmetric(10., 6.))
        .content(content)
        .on_press(open_performance);
    let framed = state_surface(button, control(), 4.);
    Tooltip::new(tip, framed).into()
}

/// Red "Picking… (Esc)" pill shown in every view while the target picker is
/// on, so the state that swallows the target's clicks can never be missed.
pub(crate) fn picking_pill(cancel: impl Fn() + 'static) -> Widget {
    state_surface(
        RawMaterialButton::new("Picking… (Esc)")
            .size(Size::new(0., 30.))
            .padding(EdgeInsets::symmetric(12., 6.))
            .label_style(super::shared::text_style(12., on_accent()).font_weight(FontWeight::W600))
            .on_press(cancel),
        danger(),
        4.,
    )
}

/// Header button that opens the Overlays menu. Its accessible name is
/// "Overlays menu" so it stays distinct from the inspector's Overlays tab.
pub(crate) fn overlays_button(active: usize, open: bool, toggle: impl Fn() + 'static) -> Widget {
    let label = if active > 0 {
        format!("Overlays · {active}")
    } else {
        "Overlays".to_owned()
    };
    let on = open || active > 0;
    let button = RawMaterialButton::new("Overlays menu")
        .size(Size::new(0., 30.))
        .padding(EdgeInsets::symmetric(10., 6.))
        .content(
            Row::new([
                ui_text(label, 12., if on { primary() } else { text_primary() }),
                gap(6., 1.),
                {
                    let chevron: Widget =
                        Icon::new(incular::widgets::internal::icons::chevron_right())
                            .size(10.)
                            .brush(text_muted())
                            .into();
                    Transform::rotation(
                        if open {
                            -std::f32::consts::FRAC_PI_2
                        } else {
                            std::f32::consts::FRAC_PI_2
                        },
                        chevron,
                    )
                    .into()
                },
            ])
            .main_axis_size(MainAxisSize::Min)
            .cross_axis_alignment(CrossAxisAlignment::Center),
        )
        .on_press(toggle);
    state_surface(button, if open { control_active() } else { control() }, 4.)
}
