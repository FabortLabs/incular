//! Real-GPU evidence that a backdrop filter's result is reused exactly when
//! nothing it reads has changed.
//!
//! The window paints a red and a blue column under a blurred glass panel
//! and, well clear of the panel, a marker box and two buttons. Toggling the
//! marker repaints the window without touching anything the blur reads: the
//! glass must look the same and must not have been filtered again. Toggling
//! the red column changes what the blur reads: the glass must show the new
//! colour, so the retained result must not have been reused.
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

use incular_core::{Color, Size};
use incular_desktop::run_application;
use incular_platform::WindowOptions;
use incular_runtime::{Application, GpuResourceSummary, Signal, Simulation, SimulationError};
use incular_widgets::internal::ActionSurface;
use incular_widgets::{
    BackdropFilter, ClipRect, Directionality, Positioned, Row, SizedBox, Stack, TextDirection,
    Widget,
};

const WIN_W: f32 = 192.0;
const WIN_H: f32 = 96.0;
/// The glass covers the left `GLASS` x `WIN_H` of the window.
const GLASS: f32 = 64.0;
/// Everything from here right is further from the glass than a blur of
/// `SIGMA` can read.
const ASIDE: f32 = 128.0;
const SIGMA: f32 = 2.0;

const RED: [u8; 4] = [255, 0, 0, 255];
const GREEN: [u8; 4] = [0, 255, 0, 255];
const BLUE: [u8; 4] = [0, 0, 255, 255];
const WHITE: [u8; 4] = [255, 255, 255, 255];
const GREY: [u8; 4] = [64, 64, 64, 255];

fn color(rgba: [u8; 4]) -> Color {
    Color::rgba(rgba[0], rgba[1], rgba[2], rgba[3])
}

fn scene(behind: &Signal<bool>, aside: &Signal<bool>) -> Widget {
    let half = Size::new(GLASS / 2., WIN_H);
    let toggle = |label: &str, signal: &Signal<bool>| {
        let signal = signal.clone();
        ActionSurface::new(label)
            .size(Size::new(WIN_W - ASIDE, 24.))
            .on_press(move || {
                let _ = signal.set(!signal.get());
            })
    };
    let children: [Widget; 6] = [
        Widget::box_(Size::new(WIN_W, WIN_H), Color::BLACK),
        Row::new([
            Widget::box_(half, color(if behind.get() { GREEN } else { RED })),
            Widget::box_(half, color(BLUE)),
        ])
        .into(),
        Positioned::new(SizedBox::from_dimensions(
            Some(GLASS),
            Some(WIN_H),
            Some(ClipRect::new(BackdropFilter::blur(
                SIGMA,
                Widget::box_(Size::new(GLASS, WIN_H), Color::TRANSPARENT),
            ))),
        ))
        .left(0.)
        .top(0.)
        .into(),
        Positioned::new(Widget::box_(
            Size::new(WIN_W - ASIDE, 24.),
            color(if aside.get() { WHITE } else { GREY }),
        ))
        .left(ASIDE)
        .top(0.)
        .into(),
        Positioned::new(toggle("Toggle aside", aside))
            .left(ASIDE)
            .top(32.)
            .into(),
        Positioned::new(toggle("Toggle behind", behind))
            .left(ASIDE)
            .top(64.)
            .into(),
    ];
    Directionality::new(TextDirection::Ltr, Stack::new(children)).into()
}

/// The pixels at logical positions.
fn pixels_at(
    simulation: &Simulation,
    points: &[(f32, f32)],
) -> Result<Vec<[u8; 4]>, SimulationError> {
    let shot = simulation.capture()?;
    let scale = shot.width() as f32 / WIN_W;
    let pixels = shot.pixels();
    Ok(points
        .iter()
        .map(|(x, y)| {
            let start = ((y * scale) as usize * shot.width() as usize + (x * scale) as usize) * 4;
            [
                pixels[start],
                pixels[start + 1],
                pixels[start + 2],
                pixels[start + 3],
            ]
        })
        .collect())
}

fn near(name: &str, pixel: [u8; 4], expected: [u8; 4]) {
    for (got, want) in pixel.iter().zip(expected) {
        assert!(
            got.abs_diff(want) <= 2,
            "{name}: got {pixel:?}, want {expected:?}"
        );
    }
}

struct Stage {
    pixels: Vec<[u8; 4]>,
    gpu: GpuResourceSummary,
}

/// Lets the frames a click schedules (press, release, hover) finish, then
/// samples the window.
fn settle(simulation: &Simulation) -> Result<Stage, SimulationError> {
    for _ in 0..4 {
        simulation.wait_for_frame()?;
    }
    std::thread::sleep(Duration::from_millis(400));
    simulation.wait_for_frame()?;
    // Deep inside each column under the glass, the boundary the blur
    // mixes, and the marker box.
    let pixels = pixels_at(
        simulation,
        &[
            (GLASS * 0.25, WIN_H / 2.),
            (GLASS * 0.5, WIN_H / 2.),
            (GLASS * 0.75, WIN_H / 2.),
            (ASIDE + 16., 12.),
        ],
    )?;
    let gpu = simulation.query_gpu_resources()?;
    Ok(Stage { pixels, gpu })
}

fn backdrop_reuse() {
    let behind = Signal::new(false);
    let aside = Signal::new(false);
    let mut application = Application::new(|_| Widget::box_(Size::new(WIN_W, WIN_H), Color::BLACK))
        .expect("bootstrap application");
    let bootstrap = application.primary_window();
    let (scene_behind, scene_aside) = (behind.clone(), aside.clone());
    let window = application
        .open_window_with(
            WindowOptions {
                initial_logical_size: Size::new(WIN_W, WIN_H),
                decorations: false,
                ..WindowOptions::new("Incular backdrop reuse")
            },
            move |_| scene(&scene_behind, &scene_aside),
        )
        .expect("open backdrop reuse window");
    assert!(application.close_window(bootstrap));

    let simulation = application.simulation().window(&window);
    let (sender, receiver) = mpsc::sync_channel(1);
    std::thread::spawn(move || {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(
            || -> Result<[Stage; 3], SimulationError> {
                let first = settle(&simulation)?;
                simulation.click("Toggle aside")?;
                let aside_toggled = settle(&simulation)?;
                simulation.click("Toggle behind")?;
                let behind_toggled = settle(&simulation)?;
                window.close().expect("close bridge remains active");
                Ok([first, aside_toggled, behind_toggled])
            },
        ));
        sender.send(result).expect("send backdrop reuse report");
    });

    let settled = Arc::new(AtomicBool::new(false));
    let watchdog_settled = settled.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_secs(180));
        if !watchdog_settled.load(Ordering::SeqCst) {
            eprintln!("backdrop_result_reuse: watchdog expired before completion");
            std::process::exit(2);
        }
    });

    let run_result = run_application(application);
    let report = receiver
        .recv_timeout(Duration::from_secs(120))
        .expect("backdrop reuse regression did not complete");
    run_result.expect("desktop event loop failed");
    let [first, aside_toggled, behind_toggled] = report
        .expect("backdrop reuse worker panicked")
        .map_err(|error| format!("backdrop reuse simulation failed: {error:?}"))
        .expect("backdrop reuse simulation failed");
    for (name, stage) in [
        ("first", &first),
        ("aside toggled", &aside_toggled),
        ("behind toggled", &behind_toggled),
    ] {
        eprintln!(
            "backdrop reuse: {name} -> {:?}, rendered {}, reused {}",
            stage.pixels, stage.gpu.backdrop_filters_rendered, stage.gpu.backdrop_filters_reused
        );
    }

    near("glass keeps the red interior", first.pixels[0], RED);
    near("glass keeps the blue interior", first.pixels[2], BLUE);
    let [red, _, blue, _] = first.pixels[1];
    assert!(
        (60..=195).contains(&red) && (60..=195).contains(&blue),
        "the blur mixes the boundary: {:?}",
        first.pixels[1]
    );
    near("marker starts grey", first.pixels[3], GREY);
    assert!(first.gpu.backdrop_filters_rendered >= 1);

    // A repaint that changes nothing the blur reads: same glass, no filter.
    near("marker repainted", aside_toggled.pixels[3], WHITE);
    assert_eq!(
        aside_toggled.pixels[..3],
        first.pixels[..3],
        "the glass is unchanged by a repaint elsewhere"
    );
    assert_eq!(
        aside_toggled.gpu.backdrop_filters_rendered, first.gpu.backdrop_filters_rendered,
        "a repaint elsewhere does not filter the backdrop again"
    );
    assert!(
        aside_toggled.gpu.backdrop_filters_reused > first.gpu.backdrop_filters_reused,
        "the repaint drew the retained result"
    );

    // A change behind the glass: the retained result is stale.
    near(
        "glass shows the new column",
        behind_toggled.pixels[0],
        GREEN,
    );
    near(
        "glass keeps the blue interior",
        behind_toggled.pixels[2],
        BLUE,
    );
    near("marker keeps its colour", behind_toggled.pixels[3], WHITE);
    assert!(
        behind_toggled.gpu.backdrop_filters_rendered > aside_toggled.gpu.backdrop_filters_rendered,
        "a change behind the glass filters the backdrop again"
    );
    settled.store(true, Ordering::SeqCst);
}

fn main() {
    if std::env::var("INCULAR_DESKTOP_LIVE_TESTS").as_deref() != Ok("1") {
        eprintln!(
            "backdrop_result_reuse: skipped live native regression; \
             set INCULAR_DESKTOP_LIVE_TESTS=1 to run"
        );
        return;
    }
    backdrop_reuse();
}
