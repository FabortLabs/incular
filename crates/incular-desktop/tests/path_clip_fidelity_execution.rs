//! Real-GPU pixel evidence for vector paths and shaped clips.
//!
//! - Filled paths are antialiased: a diagonal edge produces partially
//!   covered pixels instead of a hard staircase.
//! - A straight stroked line (zero-area path bounds) still paints inside a
//!   clip instead of being culled before stroking.
//! - `ClipRRect` and `ClipOval` cut the corners of their child rather than
//!   clipping only to the bounding rectangle.
//! - Rectangles and borders follow Skia's analytic coverage: whole-pixel
//!   edges are fully covered, and a rectangle thinner than a pixel spreads
//!   its coverage instead of snapping to a pixel row.
//! - A blur whose source is exactly its child's box stays symmetric (the first
//!   source row and column are not dropped).
//!
//! Actual GPU presentation stays native-only: this target runs solely when
//! `INCULAR_DESKTOP_LIVE_TESTS` is exactly `1` and fails on any
//! initialization error instead of passing silently.

use std::sync::mpsc;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::Duration;

use incular_core::{Color, Offset, Size};
use incular_desktop::run_application;
use incular_platform::WindowOptions;
use incular_rendering::{LineCap, Path, Stroke};
use incular_runtime::{Application, Screenshot, SimulationError};
use incular_widgets::{
    Border, ClipOval, ClipRRect, ClipRect, Container, ImageFiltered, PathView, Positioned, Stack,
    Widget,
};

const WIN: f32 = 64.0;

fn window_options(title: &str) -> WindowOptions {
    WindowOptions {
        initial_logical_size: Size::new(WIN, WIN),
        decorations: false,
        ..WindowOptions::new(title)
    }
}

fn on_black(child: Widget) -> Widget {
    Stack::new([Widget::box_(Size::new(WIN, WIN), Color::BLACK), child]).into()
}

fn white_box() -> Widget {
    Widget::box_(Size::new(WIN, WIN), Color::WHITE)
}

fn triangle() -> Widget {
    let mut builder = Path::builder();
    builder
        .move_to(Offset::new(4., 4.))
        .line_to(Offset::new(60., 4.))
        .line_to(Offset::new(4., 60.))
        .close();
    PathView::new(builder.build())
        .fill(Color::WHITE)
        .size(Size::new(WIN, WIN))
        .into()
}

fn clipped_line() -> Widget {
    let mut builder = Path::builder();
    builder
        .move_to(Offset::new(8., 32.))
        .line_to(Offset::new(56., 32.));
    ClipRect::new(
        PathView::new(builder.build())
            .stroke(
                Color::WHITE,
                Stroke {
                    width: 6.,
                    cap: LineCap::Butt,
                    ..Stroke::default()
                },
            )
            .size(Size::new(WIN, WIN)),
    )
    .into()
}

/// A solid box at a logical position; multiples of four logical pixels land
/// on device pixel boundaries at the common scales 1, 1.25, 1.5 and 2.
fn placed(left: f32, top: f32, width: f32, height: f32, color: Color) -> Widget {
    Positioned::new(Widget::box_(Size::new(width, height), color))
        .left(left)
        .top(top)
        .into()
}

/// Device pixels per logical pixel of a capture.
fn scale_of(shot: &Screenshot) -> f32 {
    shot.width() as f32 / WIN
}

fn red(shot: &Screenshot, x: u32, y: u32) -> u8 {
    shot.pixels()[(y as usize * shot.width() as usize + x as usize) * 4]
}

struct Case {
    name: &'static str,
    build: fn() -> Widget,
    check: fn(&Screenshot) -> Result<(), String>,
}

fn red_at(shot: &Screenshot, fx: f32, fy: f32) -> u8 {
    let x = ((shot.width() as f32 * fx) as u32).min(shot.width() - 1);
    let y = ((shot.height() as f32 * fy) as u32).min(shot.height() - 1);
    shot.pixels()[(y as usize * shot.width() as usize + x as usize) * 4]
}

fn cases() -> Vec<Case> {
    vec![
        Case {
            name: "filled path edges are antialiased",
            build: || on_black(triangle()),
            check: |shot| {
                let partial = shot
                    .pixels()
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .filter(|pixel| (24..=231).contains(&pixel[0]))
                    .count();
                let edge_pixels = shot.width() as usize * 56 / 64;
                if partial * 2 >= edge_pixels {
                    Ok(())
                } else {
                    Err(format!(
                        "{partial} partially covered pixels along a {edge_pixels}px diagonal"
                    ))
                }
            },
        },
        Case {
            name: "a straight stroked line paints inside a clip",
            build: || on_black(clipped_line()),
            check: |shot| match red_at(shot, 0.5, 0.5) {
                value if value >= 240 => Ok(()),
                value => Err(format!("line center red {value}")),
            },
        },
        Case {
            name: "ClipRRect cuts its corners",
            build: || on_black(ClipRRect::new(24., white_box()).into()),
            check: |shot| match (red_at(shot, 0.03, 0.03), red_at(shot, 0.5, 0.5)) {
                (corner, center) if corner <= 8 && center >= 247 => Ok(()),
                (corner, center) => Err(format!("corner red {corner}, center red {center}")),
            },
        },
        Case {
            name: "ClipOval cuts its corners",
            build: || on_black(ClipOval::new(white_box()).into()),
            check: |shot| match (red_at(shot, 0.08, 0.08), red_at(shot, 0.5, 0.5)) {
                (corner, center) if corner <= 8 && center >= 247 => Ok(()),
                (corner, center) => Err(format!("corner red {corner}, center red {center}")),
            },
        },
        Case {
            name: "whole-pixel border edges are fully covered",
            build: || {
                on_black(
                    Positioned::new(
                        Container::new()
                            .width(24.)
                            .height(24.)
                            .border(Border::new(4., Color::WHITE)),
                    )
                    .left(20.)
                    .top(20.)
                    .into(),
                )
            },
            check: |shot| {
                let scale = scale_of(shot);
                let first = (20. * scale).round() as u32;
                let last = (44. * scale).round() as u32 - 1;
                let row = (32. * scale) as u32;
                match (
                    red(shot, first - 1, row),
                    red(shot, first, row),
                    red(shot, last, row),
                ) {
                    (0..=2, 253..=255, 253..=255) => Ok(()),
                    (outside, left, right) => Err(format!(
                        "outside {outside}, first column {left}, last column {right}"
                    )),
                }
            },
        },
        Case {
            name: "a half-pixel line spreads its coverage",
            build: || on_black(placed(8., 32.25, 48., 0.5, Color::WHITE)),
            check: |shot| {
                let scale = scale_of(shot);
                let column = (32. * scale) as u32;
                let rows = (0..shot.height()).map(|y| u32::from(red(shot, column, y)));
                let total: u32 = rows.clone().sum();
                let lit = rows.filter(|value| *value > 2).count();
                let expected = 255. * 0.5 * scale;
                if (total as f32 - expected).abs() <= expected * 0.15 + 4. && lit >= 2 {
                    Ok(())
                } else {
                    Err(format!(
                        "column coverage {total} over {lit} rows, expected {expected}"
                    ))
                }
            },
        },
        Case {
            name: "a blur around its child's box stays symmetric",
            build: || {
                Stack::new([
                    Widget::box_(Size::new(WIN, WIN), Color::WHITE),
                    Positioned::new(ImageFiltered::blur(
                        3.,
                        Widget::box_(Size::new(24., 24.), Color::BLACK),
                    ))
                    .left(20.)
                    .top(20.)
                    .into(),
                ])
                .into()
            },
            check: |shot| {
                let scale = scale_of(shot);
                let left = (20. * scale).round() as u32;
                let right = (44. * scale).round() as u32;
                let row = (32. * scale) as u32;
                let mut worst = 0;
                for k in 1..=4 {
                    let a = red(shot, left - k, row);
                    let b = red(shot, right + k - 1, row);
                    worst = worst.max(a.abs_diff(b));
                }
                if worst <= 3 {
                    Ok(())
                } else {
                    Err(format!("left and right falloff differ by up to {worst}"))
                }
            },
        },
    ]
}

fn fidelity_execution() {
    let cases = cases();
    let mut application = Application::new(|_| Widget::box_(Size::new(WIN, WIN), Color::BLACK))
        .expect("bootstrap application");
    let bootstrap = application.primary_window();
    let mut windows = Vec::new();
    for (index, case) in cases.iter().enumerate() {
        let build = case.build;
        let window = application
            .open_window_with(
                window_options(&format!("Incular path fidelity {index}")),
                move |_| build(),
            )
            .expect("open fidelity window");
        windows.push(window);
    }
    assert!(application.close_window(bootstrap));

    let simulation = application.simulation();
    let (sender, receiver) = mpsc::sync_channel(1);
    let worker = std::thread::spawn(move || {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(
            || -> Result<Vec<Screenshot>, SimulationError> {
                let mut shots = Vec::new();
                for window in &windows {
                    let sim = simulation.window(window);
                    sim.wait_for_frame()?;
                    sim.wait_for_frame()?;
                    shots.push(sim.capture()?);
                    window.close().expect("close bridge remains active");
                }
                Ok(shots)
            },
        ));
        sender.send(result).expect("send fidelity report");
    });

    let settled = Arc::new(AtomicBool::new(false));
    let watchdog_settled = settled.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_secs(180));
        if !watchdog_settled.load(Ordering::SeqCst) {
            eprintln!("path_clip_fidelity_execution: watchdog expired before completion");
            std::process::exit(2);
        }
    });

    let run_result = run_application(application);
    let report = receiver
        .recv_timeout(Duration::from_secs(120))
        .expect("fidelity regression did not complete");
    worker.join().expect("fidelity worker did not panic");
    run_result.expect("desktop event loop failed");
    let shots = report
        .expect("fidelity worker panicked")
        .map_err(|error| format!("fidelity simulation failed: {error:?}"))
        .expect("fidelity simulation failed");

    let mut failures = Vec::new();
    for (case, shot) in cases.iter().zip(shots.iter()) {
        match (case.check)(shot) {
            Ok(()) => eprintln!("fidelity: {} ok", case.name),
            Err(message) => failures.push(format!("{}: {message}", case.name)),
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
    settled.store(true, Ordering::SeqCst);
}

fn main() {
    if std::env::var("INCULAR_DESKTOP_LIVE_TESTS").as_deref() != Ok("1") {
        eprintln!(
            "path_clip_fidelity_execution: skipped live native regression; \
             set INCULAR_DESKTOP_LIVE_TESTS=1 to run"
        );
        return;
    }
    fidelity_execution();
}
