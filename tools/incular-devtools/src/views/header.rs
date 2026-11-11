//! Top-bar widgets that are visible from every view: live FPS, the picker
//! indicator, the Overlays menu button and the command palette trigger.

use super::icons::IconName;
use super::shared::{
    CONTROL_HEIGHT, CONTROL_RADIUS, button_content, danger, glyph, hrow, kbd, on_accent,
    state_surface, success, text_primary, text_style, warning,
};
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
    // One bar per two pixels, newest on the right; short histories hug the
    // right edge so the graph reads as scrolling in from there.
    for (index, micros) in samples.iter().rev().take(30).enumerate() {
        let height = (*micros as f32 / ceiling * SPARK_HEIGHT).clamp(1.5, SPARK_HEIGHT);
        canvas.rrect(
            RRect::uniform(
                incular::core::Rect::from_origin_size(
                    Offset::new(SPARK_WIDTH - 2. - index as f32 * 2., SPARK_HEIGHT - height),
                    Size::new(1.5, height),
                ),
                0.75,
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
        "Frames the app presented in the last second, with the CPU time of recent frames. \
         Click to open Performance."
    };
    let color = if connected {
        fps_color(live)
    } else {
        text_muted()
    };
    let content = hrow([
        super::shared::dot(color, 7.),
        gap(8., 1.),
        Text::new(label.clone())
            .style(
                text_style(
                    12.,
                    if connected {
                        text_primary()
                    } else {
                        text_muted()
                    },
                )
                .font_weight(FontWeight::W500),
            )
            .into(),
        gap(10., 1.),
        sparkline(cpu_micros),
    ]);
    let button = RawMaterialButton::new(format!("Live frame rate: {label}"))
        .size(Size::new(0., CONTROL_HEIGHT))
        .content(button_content(content, CONTROL_HEIGHT, 12.))
        .on_press(open_performance);
    let framed = state_surface(button, control(), CONTROL_RADIUS);
    Tooltip::new(tip, framed).into()
}

/// Red "Picking… (Esc)" pill shown in every view while the target picker is
/// on, so the state that swallows the target's clicks can never be missed.
pub(crate) fn picking_pill(cancel: impl Fn() + 'static) -> Widget {
    state_surface(
        RawMaterialButton::new("Picking… (Esc)")
            .size(Size::new(0., CONTROL_HEIGHT))
            .content(button_content(
                hrow([
                    glyph(IconName::Crosshair, 14., on_accent()),
                    gap(7., 1.),
                    Text::new("Picking… (Esc)")
                        .style(text_style(12.5, on_accent()).font_weight(FontWeight::W600))
                        .into(),
                ]),
                CONTROL_HEIGHT,
                12.,
            ))
            .on_press(cancel),
        danger(),
        CONTROL_RADIUS,
    )
}

/// Header button that opens the Overlays menu. Its accessible name is
/// "Overlays menu" so it stays distinct from the inspector's Overlays tab.
pub(crate) fn overlays_button(active: usize, open: bool, toggle: impl Fn() + 'static) -> Widget {
    let on = open || active > 0;
    let mut row = vec![
        glyph(
            IconName::Layers,
            14.,
            if on { primary() } else { text_muted() },
        ),
        gap(7., 1.),
        Text::new("Overlays")
            .style(
                text_style(12.5, if on { primary() } else { text_primary() })
                    .font_weight(FontWeight::W500),
            )
            .into(),
    ];
    if active > 0 {
        row.push(gap(7., 1.));
        row.push(super::shared::count_bubble(active, primary()));
    }
    row.push(gap(6., 1.));
    row.push(glyph(IconName::ChevronDown, 12., text_muted()));
    let button = RawMaterialButton::new("Overlays menu")
        .size(Size::new(0., CONTROL_HEIGHT))
        .content(button_content(hrow(row), CONTROL_HEIGHT, 12.))
        .on_press(toggle);
    state_surface(
        button,
        if open { control_active() } else { control() },
        CONTROL_RADIUS,
    )
}

/// The top bar's search-style trigger for the command palette.
pub(crate) fn command_button(open: impl Fn() + 'static) -> Widget {
    let button = RawMaterialButton::new("Search commands (Ctrl+K)")
        .size(Size::new(0., CONTROL_HEIGHT))
        .content(button_content(
            hrow([
                glyph(IconName::Search, 14., text_muted()),
                gap(8., 1.),
                ui_text("Search commands…", 12.5, text_muted()),
                gap(28., 1.),
                kbd("Ctrl K"),
            ]),
            CONTROL_HEIGHT,
            10.,
        ))
        .on_press(open);
    state_surface(button, control(), CONTROL_RADIUS)
}
