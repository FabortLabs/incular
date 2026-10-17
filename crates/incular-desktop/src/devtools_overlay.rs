//! Compositor-only DevTools adornments drawn after the retained display list:
//! outlines, Chrome-style box highlights, and the in-app performance graph.
//!
//! Everything here emits plain `Rect` commands, which the renderer already
//! batches into its rect-instance pass; none of it touches the widget tree.

use incular_core::{Color, Offset, Rect, Size};
use incular_rendering::{DisplayList, PaintCommand};
use std::collections::VecDeque;

/// One presented frame as seen by the in-app performance graph.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FrameSample {
    pub cpu_us: u32,
    pub gpu_us: Option<u32>,
    pub budget_us: Option<u32>,
}

/// Number of frames shown by the performance overlay.
pub const PERFORMANCE_HISTORY: usize = 120;

pub(crate) fn rect([x, y, width, height]: [f32; 4]) -> Rect {
    Rect::from_origin_size(Offset::new(x, y), Size::new(width, height))
}

pub(crate) fn fill(list: &mut DisplayList, bounds: [f32; 4], color: Color) {
    if bounds[2] > 0. && bounds[3] > 0. && color.alpha > 0 {
        list.push(PaintCommand::Rect {
            rect: rect(bounds),
            color,
        });
    }
}

/// Strokes the inside edge of `bounds` with four thin rectangles.
pub(crate) fn outline(list: &mut DisplayList, bounds: [f32; 4], thickness: f32, color: Color) {
    let [x, y, width, height] = bounds;
    if width <= 0. || height <= 0. || color.alpha == 0 {
        return;
    }
    let t = thickness.min(width / 2.).min(height / 2.).max(0.);
    if t <= 0. {
        return;
    }
    fill(list, [x, y, width, t], color);
    fill(list, [x, y + height - t, width, t], color);
    let inner = height - 2. * t;
    if inner > 0. {
        fill(list, [x, y + t, t, inner], color);
        fill(list, [x + width - t, y + t, t, inner], color);
    }
}

/// Shrinks a rectangle uniformly; never inverts it.
pub(crate) fn inset(bounds: [f32; 4], amount: f32) -> [f32; 4] {
    let [x, y, width, height] = bounds;
    let amount = amount.min(width / 2.).min(height / 2.).max(0.);
    [
        x + amount,
        y + amount,
        width - 2. * amount,
        height - 2. * amount,
    ]
}

pub(crate) fn with_alpha(color: Color, alpha: f32) -> Color {
    Color::rgba(
        color.red,
        color.green,
        color.blue,
        (f32::from(color.alpha) * alpha.clamp(0., 1.)).round() as u8,
    )
}

pub(crate) fn contains(outer: [f32; 4], inner: [f32; 4]) -> bool {
    const EPS: f32 = 0.5;
    inner[0] >= outer[0] - EPS
        && inner[1] >= outer[1] - EPS
        && inner[0] + inner[2] <= outer[0] + outer[2] + EPS
        && inner[1] + inner[3] <= outer[1] + outer[3] + EPS
}

pub(crate) const RAINBOW: [Color; 7] = [
    Color::rgba(235, 64, 52, 235),
    Color::rgba(245, 150, 40, 235),
    Color::rgba(240, 214, 50, 235),
    Color::rgba(80, 200, 90, 235),
    Color::rgba(50, 180, 230, 235),
    Color::rgba(90, 100, 245, 235),
    Color::rgba(190, 90, 235, 235),
];

/// Chrome-style box highlight: padding band (green) around a content box
/// (blue) when the content box is known, otherwise one blue box, plus a thin
/// outline.
pub(crate) fn chrome_highlight(
    list: &mut DisplayList,
    bounds: [f32; 4],
    content: Option<[f32; 4]>,
) {
    const CONTENT: Color = Color::rgba(111, 168, 220, 100);
    const PADDING: Color = Color::rgba(147, 196, 125, 110);
    const EDGE: Color = Color::rgba(61, 133, 198, 230);
    let [x, y, width, height] = bounds;
    match content {
        Some(inner) if contains(bounds, inner) => {
            let [ix, iy, iw, ih] = inner;
            fill(list, [x, y, width, (iy - y).max(0.)], PADDING);
            fill(
                list,
                [x, iy + ih, width, (y + height - iy - ih).max(0.)],
                PADDING,
            );
            fill(list, [x, iy, (ix - x).max(0.), ih], PADDING);
            fill(
                list,
                [ix + iw, iy, (x + width - ix - iw).max(0.), ih],
                PADDING,
            );
            fill(list, inner, CONTENT);
            outline(list, inner, 1., with_alpha(EDGE, 0.55));
        }
        _ => fill(list, bounds, CONTENT),
    }
    outline(list, bounds, 1., EDGE);
}

/// Flutter-style frame graph in the top-right corner: CPU row above a GPU
/// row, a budget line, and over-budget frames in red.
pub(crate) fn performance_overlay(
    list: &mut DisplayList,
    window: Size,
    history: &VecDeque<FrameSample>,
) {
    const WIDTH: f32 = 240.;
    const HEIGHT: f32 = 92.;
    const MARGIN: f32 = 8.;
    const PAD: f32 = 4.;
    const ROW: f32 = 38.;
    const GOOD: Color = Color::rgba(98, 200, 140, 255);
    const BAD: Color = Color::rgba(240, 84, 84, 255);
    const GPU: Color = Color::rgba(112, 162, 255, 255);
    const LINE: Color = Color::rgba(255, 255, 255, 150);
    let x0 = (window.width - WIDTH - MARGIN).max(0.);
    let y0 = MARGIN;
    fill(list, [x0, y0, WIDTH, HEIGHT], Color::rgba(3, 4, 7, 245));
    outline(
        list,
        [x0, y0, WIDTH, HEIGHT],
        1.,
        Color::rgba(255, 255, 255, 40),
    );
    let budget = history
        .iter()
        .rev()
        .find_map(|sample| sample.budget_us)
        .unwrap_or(16_667) as f32;
    let peak = history
        .iter()
        .flat_map(|sample| [sample.cpu_us, sample.gpu_us.unwrap_or(0)])
        .max()
        .unwrap_or(0) as f32;
    let scale = (budget * 2.).max(peak.min(budget * 6.)).max(1.);
    let bar_width = (WIDTH - 2. * PAD) / PERFORMANCE_HISTORY as f32;
    let first = PERFORMANCE_HISTORY.saturating_sub(history.len());
    for (row, top) in [y0 + PAD, y0 + PAD + ROW + PAD * 2.]
        .into_iter()
        .enumerate()
    {
        fill(
            list,
            [x0 + PAD, top, WIDTH - 2. * PAD, ROW],
            Color::rgba(255, 255, 255, 14),
        );
        for (index, sample) in history.iter().enumerate() {
            let value = if row == 0 {
                Some(sample.cpu_us)
            } else {
                sample.gpu_us
            };
            let Some(value) = value else { continue };
            let over = value as f32 > budget;
            let height = (value as f32 / scale).clamp(0.02, 1.) * ROW;
            let color = if over {
                BAD
            } else if row == 0 {
                GOOD
            } else {
                GPU
            };
            let x = x0 + PAD + (first + index) as f32 * bar_width;
            fill(
                list,
                [x, top + ROW - height, (bar_width - 0.4).max(1.), height],
                color,
            );
        }
        let line = top + ROW - (budget / scale).clamp(0., 1.) * ROW;
        fill(list, [x0 + PAD, line, WIDTH - 2. * PAD, 1.], LINE);
        let latest = history.back().and_then(|sample| {
            if row == 0 {
                Some(sample.cpu_us)
            } else {
                sample.gpu_us
            }
        });
        let name = if row == 0 { "CPU" } else { "GPU" };
        let text = match latest {
            Some(us) => format!("{name} {:.1} ms", us as f32 / 1000.),
            None => format!("{name} n/a"),
        };
        draw_text(
            list,
            &text,
            9.,
            Color::rgba(255, 255, 255, 235),
            x0 + PAD + 3.,
            top + 1.,
        );
    }
}

/// Small dark chip with `text` just above `anchor` (below the top edge when
/// there is no room), clamped inside the window.
pub(crate) fn label_chip(list: &mut DisplayList, window: Size, anchor: [f32; 4], text: &str) {
    const SIZE: f32 = 11.;
    const PAD_X: f32 = 6.;
    const PAD_Y: f32 = 3.;
    let (commands, text_size) = incular_widgets::devtools::label_commands(
        text,
        SIZE,
        Color::rgba(255, 255, 255, 255),
        Offset::new(0., 0.),
    );
    let width = text_size.width + 2. * PAD_X;
    let height = text_size.height + 2. * PAD_Y;
    let x = anchor[0].clamp(0., (window.width - width).max(0.));
    let above = anchor[1] - height - 3.;
    let below = anchor[1] + anchor[3] + 3.;
    let y = if above >= 0. {
        above
    } else if below + height <= window.height {
        below
    } else {
        (anchor[1] + 3.).min((window.height - height).max(0.))
    };
    fill(list, [x, y, width, height], Color::rgba(8, 10, 15, 245));
    outline(
        list,
        [x, y, width, height],
        1.,
        Color::rgba(61, 133, 198, 230),
    );
    list.push(PaintCommand::PushTransform {
        transform: incular_core::Transform::translation(Offset::new(x + PAD_X, y + PAD_Y)),
    });
    for command in commands {
        // The label helper positions at its own origin; skip its transform pair.
        if matches!(
            command,
            PaintCommand::PushTransform { .. } | PaintCommand::PopTransform
        ) {
            continue;
        }
        list.push(command);
    }
    list.push(PaintCommand::PopTransform);
}

fn draw_text(list: &mut DisplayList, text: &str, size: f32, color: Color, x: f32, y: f32) {
    let (commands, _) =
        incular_widgets::devtools::label_commands(text, size, color, Offset::new(x, y));
    for command in commands {
        list.push(command);
    }
}
