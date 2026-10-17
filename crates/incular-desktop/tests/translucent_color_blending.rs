//! Real-GPU pixel evidence for the compositing color space.
//!
//! Each window paints an opaque background and a translucent rectangle over
//! it, and the test asserts the presented center pixel. Incular blends
//! encoded sRGB components like Flutter, browsers and CSS, so a dark
//! translucent panel over a light page stays dark: `rgba(3, 4, 7, 245)` over
//! a cream page is about 13, where blending in linear light would show a
//! mid-grey 54.
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
use incular_runtime::{Application, Simulation, SimulationError};
use incular_widgets::{Stack, Widget};

const WIN_W: f32 = 64.0;
const WIN_H: f32 = 64.0;

fn window_options(title: &str) -> WindowOptions {
    WindowOptions {
        initial_logical_size: Size::new(WIN_W, WIN_H),
        decorations: false,
        ..WindowOptions::new(title)
    }
}

fn layered_window(layers: &[Color]) -> Widget {
    Stack::new(
        layers
            .iter()
            .map(|color| Widget::box_(Size::new(WIN_W, WIN_H), *color)),
    )
    .into()
}

fn center_pixel(simulation: &Simulation) -> Result<[u8; 4], SimulationError> {
    let shot = simulation.capture()?;
    let start =
        ((shot.height() / 2) as usize * shot.width() as usize + (shot.width() / 2) as usize) * 4;
    let pixels = shot.pixels();
    Ok([
        pixels[start],
        pixels[start + 1],
        pixels[start + 2],
        pixels[start + 3],
    ])
}

/// Straight source-over of each layer onto the previous result, computed on
/// encoded sRGB bytes the way Flutter and browsers composite.
fn encoded_source_over(layers: &[Color]) -> [u8; 4] {
    let mut result = [0f32; 3];
    for layer in layers {
        let alpha = f32::from(layer.alpha) / 255.;
        for (channel, source) in result.iter_mut().zip([layer.red, layer.green, layer.blue]) {
            *channel = f32::from(source) * alpha + *channel * (1. - alpha);
        }
    }
    let [red, green, blue] = result.map(|channel| channel.round() as u8);
    [red, green, blue, 255]
}

fn cases() -> Vec<(&'static str, Vec<Color>)> {
    let cream = Color::rgba(247, 243, 235, 255);
    vec![
        (
            "near-black panel over cream",
            vec![cream, Color::rgba(3, 4, 7, 245)],
        ),
        (
            "faint white row over the panel",
            vec![
                cream,
                Color::rgba(3, 4, 7, 245),
                Color::rgba(255, 255, 255, 14),
            ],
        ),
        (
            "dark translucent surface over white",
            vec![Color::WHITE, Color::rgba(20, 20, 24, 220)],
        ),
        (
            "half black over white is encoded mid-grey",
            vec![Color::WHITE, Color::rgba(0, 0, 0, 128)],
        ),
        (
            "half red over blue",
            vec![Color::rgba(0, 0, 255, 255), Color::rgba(255, 0, 0, 128)],
        ),
    ]
}

fn blending_execution() {
    let cases = cases();
    let mut application = Application::new(|_| Widget::box_(Size::new(WIN_W, WIN_H), Color::BLACK))
        .expect("bootstrap application");
    let bootstrap = application.primary_window();
    let mut windows = Vec::new();
    for (index, (_, layers)) in cases.iter().enumerate() {
        let layers = layers.clone();
        windows.push(
            application
                .open_window_with(
                    window_options(&format!("Incular translucent blending {index}")),
                    move |_| layered_window(&layers),
                )
                .expect("open blending window"),
        );
    }
    assert!(application.close_window(bootstrap));

    let simulation = application.simulation();
    let (sender, receiver) = mpsc::sync_channel(1);
    std::thread::spawn(move || {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(
            || -> Result<Vec<[u8; 4]>, SimulationError> {
                let mut pixels = Vec::new();
                for window in &windows {
                    let sim = simulation.window(window);
                    sim.wait_for_frame()?;
                    sim.wait_for_frame()?;
                    pixels.push(center_pixel(&sim)?);
                    window.close().expect("close bridge remains active");
                }
                Ok(pixels)
            },
        ));
        sender.send(result).expect("send blending report");
    });

    let settled = Arc::new(AtomicBool::new(false));
    let watchdog_settled = settled.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_secs(180));
        if !watchdog_settled.load(Ordering::SeqCst) {
            eprintln!("translucent_color_blending: watchdog expired before completion");
            std::process::exit(2);
        }
    });

    let run_result = run_application(application);
    let report = receiver
        .recv_timeout(Duration::from_secs(120))
        .expect("blending regression did not complete");
    run_result.expect("desktop event loop failed");
    let pixels = report
        .expect("blending worker panicked")
        .map_err(|error| format!("blending simulation failed: {error:?}"))
        .expect("blending simulation failed");

    assert_eq!(pixels.len(), cases.len());
    for ((name, layers), pixel) in cases.iter().zip(pixels.iter()) {
        let expected = encoded_source_over(layers);
        eprintln!("blend: {name} -> {pixel:?} (want {expected:?})");
        // One 8-bit step per blend: fixed-function blending rounds each
        // intermediate result into the framebuffer.
        let tolerance = layers.len() as u8 - 1;
        for (got, want) in pixel.iter().zip(expected.iter()) {
            assert!(
                got.abs_diff(*want) <= tolerance,
                "{name}: got {pixel:?}, want {expected:?} (tolerance {tolerance})"
            );
        }
    }
    settled.store(true, Ordering::SeqCst);
}

fn main() {
    if std::env::var("INCULAR_DESKTOP_LIVE_TESTS").as_deref() != Ok("1") {
        eprintln!(
            "translucent_color_blending: skipped live native regression; \
             set INCULAR_DESKTOP_LIVE_TESTS=1 to run"
        );
        return;
    }
    blending_execution();
}
