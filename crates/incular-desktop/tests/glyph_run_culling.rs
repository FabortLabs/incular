//! Real-GPU evidence that clip culling keeps partly visible glyph runs.
//!
//! The renderer skips glyph runs wholly outside the active clip rectangle.
//! This window clips a large line whose baseline lies below the clip while
//! the upper part of its ink lies inside it; the visible band must still
//! contain glyph pixels, and everything below the clip must stay clear.
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
use incular_runtime::{Application, SimulationError};
use incular_widgets::{
    ClipRect, Directionality, Positioned, SizedBox, Stack, Text, TextDirection, TextStyle, Widget,
};

const WIN: f32 = 120.0;
const CLIP_HEIGHT: f32 = 60.0;
// The line box starts 20 logical pixels above the clip edge, so its
// baseline (about 37 px lower) is outside the clip and only the top of
// each capital is visible.
const LINE_TOP: f32 = 40.0;

fn scene() -> Widget {
    let style = TextStyle::default().font_size(40.0).color(Color::BLACK);
    let clipped = ClipRect::new(Stack::new([Positioned::new(
        Text::new("MMMM").style(style).soft_wrap(false),
    )
    .left(4.0)
    .top(LINE_TOP)]));
    Directionality::new(
        TextDirection::Ltr,
        Stack::new([
            Widget::box_(Size::new(WIN, WIN), Color::WHITE),
            Positioned::new(SizedBox::from_dimensions(
                Some(WIN),
                Some(CLIP_HEIGHT),
                Some(clipped),
            ))
            .left(0.0)
            .into(),
        ]),
    )
    .into()
}

fn dark_pixels(pixels: &[u8], width: u32, scale: f32, rows: std::ops::Range<f32>) -> usize {
    let first = (rows.start * scale).ceil() as usize;
    let last = (rows.end * scale).floor() as usize;
    (first..last)
        .flat_map(|y| (0..width as usize).map(move |x| (y * width as usize + x) * 4))
        .filter(|&offset| pixels[offset] < 128 && pixels[offset + 1] < 128)
        .count()
}

fn culling_execution() {
    let application = Application::new_with_options(
        WindowOptions {
            initial_logical_size: Size::new(WIN, WIN),
            decorations: false,
            ..WindowOptions::new("Incular glyph run culling")
        },
        |_| scene(),
    )
    .expect("application");
    let window = application
        .window_handle(application.primary_window())
        .expect("primary window handle");
    let simulation = application.simulation().window(&window);
    let (sender, receiver) = mpsc::sync_channel(1);
    let worker = std::thread::spawn(move || {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(
            || -> Result<(usize, usize), SimulationError> {
                simulation.wait_for_frame()?;
                simulation.wait_for_frame()?;
                let shot = simulation.capture()?;
                let scale = shot.width() as f32 / WIN;
                let visible = dark_pixels(
                    shot.pixels(),
                    shot.width(),
                    scale,
                    LINE_TOP..CLIP_HEIGHT - 1.0,
                );
                let clipped =
                    dark_pixels(shot.pixels(), shot.width(), scale, CLIP_HEIGHT + 1.0..WIN);
                window.close().expect("close bridge remains active");
                Ok((visible, clipped))
            },
        ));
        sender.send(result).expect("send culling report");
    });

    let settled = Arc::new(AtomicBool::new(false));
    let watchdog_settled = settled.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_secs(180));
        if !watchdog_settled.load(Ordering::SeqCst) {
            eprintln!("glyph_run_culling: watchdog expired before completion");
            std::process::exit(2);
        }
    });

    let run_result = run_application(application);
    let report = receiver
        .recv_timeout(Duration::from_secs(120))
        .expect("culling regression did not complete");
    worker.join().expect("culling worker did not panic");
    run_result.expect("desktop event loop failed");
    let (visible, clipped) = report
        .expect("culling worker panicked")
        .map_err(|error| format!("culling simulation failed: {error:?}"))
        .expect("culling simulation failed");
    eprintln!("glyph_run_culling: visible={visible} clipped={clipped}");
    assert!(
        visible > 0,
        "the visible top of a run whose baseline is clipped must still draw"
    );
    assert_eq!(clipped, 0, "glyph ink below the clip must stay clipped");
    settled.store(true, Ordering::SeqCst);
}

fn main() {
    if std::env::var("INCULAR_DESKTOP_LIVE_TESTS").as_deref() != Ok("1") {
        eprintln!(
            "glyph_run_culling: skipped live native regression; \
             set INCULAR_DESKTOP_LIVE_TESTS=1 to run"
        );
        return;
    }
    culling_execution();
}
