//! Real-GPU pixel evidence for backdrop filters.
//!
//! Each window paints a red left half and a blue right half, then a
//! full-window `BackdropFilter` with a transparent child. A blur mixes the
//! halves only near their boundary; a matrix translation moves the backdrop
//! right, so red shows where blue was painted, and the strip it uncovers
//! shows the backdrop unfiltered.
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

use incular_core::{Color, Offset, Size, Transform};
use incular_desktop::run_application;
use incular_platform::WindowOptions;
use incular_runtime::{Application, Simulation, SimulationError, WindowHandle};
use incular_widgets::{BackdropFilter, Directionality, Row, Stack, TextDirection, Widget};

const WIN_W: f32 = 64.0;
const WIN_H: f32 = 64.0;
const RED: [u8; 4] = [255, 0, 0, 255];
const BLUE: [u8; 4] = [0, 0, 255, 255];

fn window_options(title: &str) -> WindowOptions {
    WindowOptions {
        initial_logical_size: Size::new(WIN_W, WIN_H),
        decorations: false,
        ..WindowOptions::new(title)
    }
}

fn filtered_halves(filter: fn(Widget) -> BackdropFilter) -> Widget {
    let half = Size::new(WIN_W / 2., WIN_H);
    let children: [Widget; 2] = [
        Row::new([
            Widget::box_(half, Color::rgba(255, 0, 0, 255)),
            Widget::box_(half, Color::rgba(0, 0, 255, 255)),
        ])
        .into(),
        filter(Widget::box_(Size::new(WIN_W, WIN_H), Color::TRANSPARENT)).into(),
    ];
    Directionality::new(TextDirection::Ltr, Stack::new(children)).into()
}

/// The pixels at logical x positions along the middle row.
fn row_pixels(simulation: &Simulation, xs: &[f32]) -> Result<Vec<[u8; 4]>, SimulationError> {
    let shot = simulation.capture()?;
    let scale = shot.width() as f32 / WIN_W;
    let y = (shot.height() / 2) as usize;
    let pixels = shot.pixels();
    Ok(xs
        .iter()
        .map(|x| {
            let start = (y * shot.width() as usize + (x * scale) as usize) * 4;
            [
                pixels[start],
                pixels[start + 1],
                pixels[start + 2],
                pixels[start + 3],
            ]
        })
        .collect())
}

struct Case {
    name: &'static str,
    filter: fn(Widget) -> BackdropFilter,
    xs: &'static [f32],
    check: fn(&[[u8; 4]]),
}

fn near(name: &str, pixel: [u8; 4], expected: [u8; 4]) {
    for (got, want) in pixel.iter().zip(expected) {
        assert!(
            got.abs_diff(want) <= 2,
            "{name}: got {pixel:?}, want {expected:?}"
        );
    }
}

fn cases() -> Vec<Case> {
    vec![
        Case {
            name: "blur",
            filter: |child| BackdropFilter::blur(4., child),
            // 16 logical pixels is four sigma from both the boundary and
            // the window edge.
            xs: &[16., 32., 48.],
            check: |pixels| {
                near("blur keeps the red interior", pixels[0], RED);
                near("blur keeps the blue interior", pixels[2], BLUE);
                let [red, green, blue, alpha] = pixels[1];
                assert!(
                    (80..=175).contains(&red) && (80..=175).contains(&blue),
                    "blur mixes the boundary: {:?}",
                    pixels[1]
                );
                assert!(green <= 2 && alpha == 255, "boundary {:?}", pixels[1]);
            },
        },
        Case {
            name: "matrix",
            filter: |child| {
                BackdropFilter::matrix(Transform::translation(Offset::new(16., 0.)), child)
            },
            xs: &[8., 40., 56.],
            check: |pixels| {
                // The uncovered strip shows the unfiltered backdrop.
                near("matrix uncovers the backdrop", pixels[0], RED);
                near("matrix moves red right", pixels[1], RED);
                near("matrix keeps blue further right", pixels[2], BLUE);
            },
        },
    ]
}

fn backdrop_execution() {
    let cases = cases();
    let mut application = Application::new(|_| Widget::box_(Size::new(WIN_W, WIN_H), Color::BLACK))
        .expect("bootstrap application");
    let bootstrap = application.primary_window();
    let mut windows: Vec<WindowHandle> = Vec::new();
    for case in &cases {
        let filter = case.filter;
        windows.push(
            application
                .open_window_with(
                    window_options(&format!("Incular backdrop {}", case.name)),
                    move |_| filtered_halves(filter),
                )
                .expect("open backdrop window"),
        );
    }
    assert!(application.close_window(bootstrap));

    let simulation = application.simulation();
    let samples: Vec<&'static [f32]> = cases.iter().map(|case| case.xs).collect();
    let (sender, receiver) = mpsc::sync_channel(1);
    std::thread::spawn(move || {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(
            || -> Result<Vec<Vec<[u8; 4]>>, SimulationError> {
                let mut pixels = Vec::new();
                for (window, xs) in windows.iter().zip(&samples) {
                    let sim = simulation.window(window);
                    sim.wait_for_frame()?;
                    sim.wait_for_frame()?;
                    pixels.push(row_pixels(&sim, xs)?);
                    window.close().expect("close bridge remains active");
                }
                Ok(pixels)
            },
        ));
        sender.send(result).expect("send backdrop report");
    });

    let settled = Arc::new(AtomicBool::new(false));
    let watchdog_settled = settled.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_secs(180));
        if !watchdog_settled.load(Ordering::SeqCst) {
            eprintln!("backdrop_filter_execution: watchdog expired before completion");
            std::process::exit(2);
        }
    });

    let run_result = run_application(application);
    let report = receiver
        .recv_timeout(Duration::from_secs(120))
        .expect("backdrop regression did not complete");
    run_result.expect("desktop event loop failed");
    let pixels = report
        .expect("backdrop worker panicked")
        .map_err(|error| format!("backdrop simulation failed: {error:?}"))
        .expect("backdrop simulation failed");

    assert_eq!(pixels.len(), cases.len());
    for (case, pixels) in cases.iter().zip(&pixels) {
        eprintln!("backdrop: {} -> {pixels:?}", case.name);
        (case.check)(pixels);
    }
    settled.store(true, Ordering::SeqCst);
}

fn main() {
    if std::env::var("INCULAR_DESKTOP_LIVE_TESTS").as_deref() != Ok("1") {
        eprintln!(
            "backdrop_filter_execution: skipped live native regression; \
             set INCULAR_DESKTOP_LIVE_TESTS=1 to run"
        );
        return;
    }
    backdrop_execution();
}
