//! Real-GPU pixel evidence for vector paths and shaped clips.
//!
//! - Filled paths are antialiased: a diagonal edge produces partially
//!   covered pixels instead of a hard staircase.
//! - A straight stroked line (zero-area path bounds) still paints inside a
//!   clip instead of being culled before stroking.
//! - `ClipRRect` and `ClipOval` cut the corners of their child rather than
//!   clipping only to the bounding rectangle.
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
use incular_widgets::{ClipOval, ClipRRect, ClipRect, PathView, Stack, Widget};

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
