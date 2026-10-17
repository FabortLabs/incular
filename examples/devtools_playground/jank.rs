//! Section 2: deliberate jank, rebuild storms and paint-only churn.
use super::theme::*;
use incular::controls_prelude::{Checkbox, Switch};
use incular::prelude::*;
use std::time::{Duration, Instant};

/// How the 10x10 cell grid changes every frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GridMode {
    /// Built once; nothing changes.
    Still,
    /// 100 widgets are rebuilt (new colour) every frame.
    RebuildStorm,
    /// One `CustomPaint` re-records its colours; no widget rebuilds.
    RepaintOnly,
}

#[derive(Clone)]
pub struct Jank {
    pub simulate: Signal<bool>,
    pub mode: Signal<GridMode>,
    tick: AnimationController,
}

impl Jank {
    pub fn new() -> Self {
        Self {
            simulate: Signal::new(false),
            mode: Signal::new(GridMode::Still),
            tick: AnimationController::new(Duration::from_millis(1600)),
        }
    }
}

/// Burns about `millis` of wall-clock time on the calling (UI) thread.
fn busy_wait(millis: u64) -> u64 {
    let start = Instant::now();
    let mut acc = 0_u64;
    while start.elapsed() < Duration::from_millis(millis) {
        acc = acc.wrapping_mul(6364136223846793005).wrapping_add(1);
    }
    acc
}

const CELL: f32 = 26.;
const SIDE: usize = 10;

fn cell_color(index: usize, phase: f32) -> Color {
    hue(phase + index as f32 / (SIDE * SIDE) as f32)
}

fn storm_grid(phase: f32) -> Widget {
    let rows = (0..SIDE).map(|row| {
        Row::new((0..SIDE).map(move |column| {
            let index = row * SIDE + column;
            Container::new()
                .width(CELL)
                .height(CELL)
                .margin(EdgeInsets::all(1.))
                .radius(4.)
                .color(cell_color(index, phase))
        }))
        .into()
    });
    Column::new(rows.collect::<Vec<Widget>>()).into()
}

fn paint_grid(phase: f32) -> Widget {
    let mut list = DisplayList::default();
    for row in 0..SIDE {
        for column in 0..SIDE {
            let index = row * SIDE + column;
            list.push(incular::rendering::PaintCommand::Rect {
                rect: Rect::from_origin_size(
                    Offset::new(
                        column as f32 * (CELL + 2.) + 1.,
                        row as f32 * (CELL + 2.) + 1.,
                    ),
                    Size::new(CELL, CELL),
                ),
                color: cell_color(index, phase),
            });
        }
    }
    RepaintBoundary::new(CustomPaint::new(
        Size::new(SIDE as f32 * (CELL + 2.), SIDE as f32 * (CELL + 2.)),
        list,
    ))
    .into()
}

pub fn view(jank: &Jank) -> Widget {
    let simulate = jank.simulate.clone();
    let simulate_toggle = jank.simulate.clone();
    let jank_card = card(
        "Simulate jank",
        "Busy-loops about 25 ms inside the build of the widget below, every frame, while on.",
        420.,
        Column::new([
            Row::new([
                Widget::from(Switch::new(simulate.get()).on_changed(move |on| {
                    simulate_toggle.set(on);
                }))
                .accessibility_label("Simulate jank"),
                hgap(12.),
                body(if simulate.get() {
                    "Jank is ON"
                } else {
                    "Jank is off"
                }),
            ])
            .into(),
            gap(14.),
            ticking(&jank.tick, false, {
                let simulate = jank.simulate.clone();
                move |value| {
                    let burned = if simulate.get() { busy_wait(25) } else { 0 };
                    let _ = burned;
                    Container::new()
                        .padding(EdgeInsets::all(12.))
                        .radius(10.)
                        .color(if simulate.get() { ROSE } else { TEAL })
                        .child(text(
                            format!("janky widget · phase {value:.2}"),
                            13.,
                            FontWeight::W600,
                            SURFACE,
                        ))
                        .into()
                }
            }),
        ])
        .cross_axis_alignment(CrossAxisAlignment::Start),
    );

    let mode = jank.mode.get();
    let choose = |label: &'static str, target: GridMode| {
        let signal = jank.mode.clone();
        Widget::from(
            Checkbox::new(mode == target)
                .label(label)
                .on_changed(move |on| {
                    signal.set(if on { target } else { GridMode::Still });
                }),
        )
    };
    let grid = match mode {
        GridMode::Still => storm_grid(0.),
        GridMode::RebuildStorm => ticking(&jank.tick, false, storm_grid),
        GridMode::RepaintOnly => ticking(&jank.tick, false, |phase| {
            // The ticking wrapper rebuilds one stateful builder; the CustomPaint
            // inside changes only its recorded colours.
            paint_grid(phase)
        }),
    };
    let grid_card = card(
        "Rebuild storm / repaint only",
        "Pick a mode, then turn on rebuild flashes or the repaint rainbow in DevTools Overlays.",
        420.,
        Column::new([
            Row::new([
                choose("Rebuild storm", GridMode::RebuildStorm),
                hgap(18.),
                choose("Repaint only", GridMode::RepaintOnly),
            ])
            .into(),
            gap(14.),
            body(match mode {
                GridMode::Still => "Still: nothing changes",
                GridMode::RebuildStorm => "100 cells rebuilt every frame",
                GridMode::RepaintOnly => "One custom painter re-recorded every frame",
            }),
            gap(10.),
            grid,
        ])
        .cross_axis_alignment(CrossAxisAlignment::Start),
    );
    page(
        "Jank lab",
        "Provoke slow frames, heavy rebuilds and paint-only changes on demand.",
        vec![jank_card, grid_card],
    )
}
