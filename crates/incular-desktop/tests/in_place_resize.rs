use incular_config::WindowSizePolicy;
use incular_core::{Color, Size};
use incular_desktop::run_application;
use incular_platform::WindowOptions;
use incular_runtime::{Application, Simulation, SimulationError};
use incular_widgets::internal::ActionSurface;
use incular_widgets::{SizedBox, Widget};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::time::{Duration, Instant};

const WIDTH: f32 = 184.0;
const COMPACT_HEIGHT: f32 = 54.0;
const EXPANDED_HEIGHT: f32 = 154.0;

#[derive(Debug)]
struct ResizeCaptureReport {
    initial: (u32, u32),
    expanded: (u32, u32),
    compact: (u32, u32),
}

fn window_options(size_policy: WindowSizePolicy) -> WindowOptions {
    WindowOptions {
        initial_logical_size: Size::new(WIDTH, COMPACT_HEIGHT),
        size_policy,
        decorations: false,
        ..WindowOptions::new("Incular in-place resize regression")
    }
}

fn settle(simulation: &Simulation) -> Result<(), SimulationError> {
    // Drain native focus/configure transitions before measuring a requested
    // size. The bounded helpers below verify that the new size then remains
    // stable across multiple presented frames.
    for _ in 0..3 {
        simulation.wait_for_frame()?;
    }
    Ok(())
}

fn wait_for_focus(
    simulation: &Simulation,
    focused: &AtomicBool,
    label: &str,
) -> Result<(), SimulationError> {
    let deadline = Instant::now() + Duration::from_secs(5);
    while !focused.load(Ordering::Acquire) {
        if Instant::now() >= deadline {
            return Err(SimulationError::FrameFailed(format!(
                "{label} did not report native focus within 5s"
            )));
        }
        simulation.wait_for_frame()?;
    }
    Ok(())
}

fn dimensions(simulation: &Simulation) -> Result<(u32, u32), SimulationError> {
    let capture = simulation.capture()?;
    Ok((capture.width(), capture.height()))
}

fn dimensions_until_stable(
    simulation: &Simulation,
    label: &str,
) -> Result<(u32, u32), SimulationError> {
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut last = dimensions(simulation)?;
    let mut matching_frames = 1;
    while matching_frames < 3 {
        if Instant::now() >= deadline {
            return Err(SimulationError::FrameFailed(format!(
                "{label} native size did not remain stable for 3 frames within 5s; last capture was {last:?}"
            )));
        }
        simulation.wait_for_frame()?;
        let current = dimensions(simulation)?;
        if current == last {
            matching_frames += 1;
        } else {
            last = current;
            matching_frames = 1;
        }
    }
    Ok(last)
}

fn expected_height(initial_width: u32, logical_height: f32) -> u32 {
    let scale_factor = f64::from(initial_width) / f64::from(WIDTH);
    (f64::from(logical_height) * scale_factor).round() as u32
}

fn resize_tolerance(initial_width: u32) -> u32 {
    let scale_factor = f64::from(initial_width) / f64::from(WIDTH);
    // An integral inferred scale has no compositor rounding allowance. At a
    // fractional scale, one logical pixel can move the physical buffer edge.
    let nearest_integral_scale = scale_factor.round().max(1.0);
    let scale_rounding_quantum = 1.0 / f64::from(WIDTH);
    if (scale_factor - nearest_integral_scale).abs() < scale_rounding_quantum {
        0
    } else {
        scale_factor.ceil().max(1.0) as u32
    }
}

fn dimensions_until_height(
    simulation: &Simulation,
    initial_width: u32,
    logical_height: f32,
    label: &str,
) -> Result<(u32, u32), SimulationError> {
    let expected = (
        initial_width,
        expected_height(initial_width, logical_height),
    );
    let tolerance = resize_tolerance(initial_width);
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut last = dimensions(simulation)?;
    let mut matching_frames = 0;
    loop {
        if last.0.abs_diff(expected.0) <= tolerance && last.1.abs_diff(expected.1) <= tolerance {
            matching_frames += 1;
            if matching_frames == 3 {
                return Ok(last);
            }
        } else {
            matching_frames = 0;
        }
        if Instant::now() >= deadline {
            return Err(SimulationError::FrameFailed(format!(
                "{label} did not remain at expected physical size {expected:?} ±{tolerance}px for 3 frames within 5s; last capture was {last:?}"
            )));
        }
        simulation.wait_for_frame()?;
        last = dimensions(simulation)?;
    }
}

fn assert_within(actual: u32, expected: u32, tolerance: u32, what: &str) {
    assert!(
        actual.abs_diff(expected) <= tolerance,
        "{what}: expected {expected}±{tolerance} physical pixels, got {actual}"
    );
}

fn assert_report(report: &ResizeCaptureReport) {
    let scale_factor = f64::from(report.initial.0) / f64::from(WIDTH);
    // Fractional-scale compositors can acknowledge a logical width using
    // their integer logical geometry, which moves the buffer edge by up to
    // one logical pixel. Keep integral-scale backends exact; at a fractional
    // scale, bound that negotiation in physical pixels while still rejecting
    // a real width change.
    let tolerance = resize_tolerance(report.initial.0);
    assert_within(
        report.expanded.0,
        report.initial.0,
        tolerance,
        "expanded surface width",
    );
    assert_within(
        report.compact.0,
        report.initial.0,
        tolerance,
        "compact surface width",
    );
    assert_within(
        report.initial.1,
        expected_height(report.initial.0, COMPACT_HEIGHT),
        tolerance,
        "initial compact surface height",
    );
    assert_within(
        report.expanded.1,
        expected_height(report.initial.0, EXPANDED_HEIGHT),
        tolerance,
        "post-resize expanded surface height",
    );
    assert_within(
        report.compact.1,
        expected_height(report.initial.0, COMPACT_HEIGHT),
        tolerance,
        "shrinking in place must restore the compact physical surface",
    );
    let expected_height_change =
        (f64::from(EXPANDED_HEIGHT - COMPACT_HEIGHT) * scale_factor).round() as u32;
    assert_within(
        report.expanded.1.abs_diff(report.compact.1),
        expected_height_change,
        tolerance * 2,
        "expanded-to-compact height change",
    );
}

fn programmatic_and_content_resizes_update_capture_surfaces_in_place() {
    let expanded = incular_runtime::Signal::new(false);
    let observed = expanded.clone();
    let content_focused = Arc::new(AtomicBool::new(false));
    let focus_for_content = Arc::clone(&content_focused);
    let mut application =
        Application::new(|_| SizedBox::new().into()).expect("bootstrap application");
    let bootstrap = application.primary_window();
    let explicit = application
        .open_window_with(window_options(WindowSizePolicy::Viewport), |_| {
            Widget::box_(Size::new(WIDTH, COMPACT_HEIGHT), Color::TRANSPARENT)
        })
        .expect("open explicit resize target");
    let content = application
        .open_window_with(window_options(WindowSizePolicy::Content), move |context| {
            focus_for_content.store(context.window_focused(), Ordering::Release);
            let is_expanded = observed.get();
            let state = observed.clone();
            let button = ActionSurface::new(if is_expanded { "Shrink" } else { "Expand" })
                .size(Size::new(96.0, 40.0))
                .on_press(move || {
                    let _ = state.set(!state.get());
                });
            SizedBox::new()
                .width(WIDTH)
                .height(if is_expanded {
                    EXPANDED_HEIGHT
                } else {
                    COMPACT_HEIGHT
                })
                .child(button)
                .into()
        })
        .expect("open content-sized resize target");
    assert!(application.close_window(bootstrap));
    let active = application.active_window_ids();
    assert_eq!(active.len(), 2);
    assert!(active.contains(&explicit.id()));
    assert!(active.contains(&content.id()));

    let root_simulation = application.simulation();
    let explicit_simulation = root_simulation.window(&explicit);
    let content_simulation = root_simulation.window(&content);
    assert_eq!(explicit_simulation.window_id(), explicit.id());
    assert_eq!(content_simulation.window_id(), content.id());

    let explicit_handle = explicit.clone();
    let content_handle = content.clone();
    let content_focused = Arc::clone(&content_focused);
    let (sender, receiver) = mpsc::sync_channel(1);
    let worker = std::thread::spawn(move || {
        let result = (|| -> Result<(ResizeCaptureReport, ResizeCaptureReport), SimulationError> {
            // The last opened native window becomes active. BuildContext
            // observes that compositor focus transition, including the
            // associated initial configure changes for both windows. Wait for
            // it and for both native sizes to settle before requesting a new
            // size, so a late map/focus configure cannot overwrite the test's
            // resize operation.
            wait_for_focus(
                &content_simulation,
                &content_focused,
                "content resize window",
            )?;
            settle(&explicit_simulation)?;
            settle(&content_simulation)?;
            let initial = dimensions_until_stable(&explicit_simulation, "explicit resize window")?;
            let _ = dimensions_until_stable(&content_simulation, "content resize window")?;
            assert!(
                explicit_handle
                    .request_logical_size(Size::new(WIDTH, EXPANDED_HEIGHT))
                    .is_ok()
            );
            let expanded = dimensions_until_height(
                &explicit_simulation,
                initial.0,
                EXPANDED_HEIGHT,
                "explicit window expansion",
            )?;
            assert_eq!(explicit_simulation.window_id(), explicit_handle.id());
            assert!(
                explicit_handle
                    .request_logical_size(Size::new(WIDTH, COMPACT_HEIGHT))
                    .is_ok()
            );
            let compact = dimensions_until_height(
                &explicit_simulation,
                initial.0,
                COMPACT_HEIGHT,
                "explicit window shrink",
            )?;
            assert_eq!(explicit_simulation.window_id(), explicit_handle.id());
            let explicit_report = ResizeCaptureReport {
                initial,
                expanded,
                compact,
            };

            let initial = dimensions_until_stable(&content_simulation, "content resize window")?;
            content_simulation.click("Expand")?;
            let expanded = dimensions_until_height(
                &content_simulation,
                initial.0,
                EXPANDED_HEIGHT,
                "content window expansion",
            )?;
            assert_eq!(content_simulation.window_id(), content_handle.id());
            content_simulation.click("Shrink")?;
            let compact = dimensions_until_height(
                &content_simulation,
                initial.0,
                COMPACT_HEIGHT,
                "content window shrink",
            )?;
            assert_eq!(content_simulation.window_id(), content_handle.id());
            let content_report = ResizeCaptureReport {
                initial,
                expanded,
                compact,
            };
            Ok((explicit_report, content_report))
        })();

        // Close through the same generational handles. No resize path is
        // allowed to replace either native window or its Incular identity.
        let _ = explicit_handle.close();
        let _ = content_handle.close();
        sender.send(result).expect("send resize reports");
    });

    let run_result = run_application(application);
    let (explicit_report, content_report) = receiver
        .recv_timeout(Duration::from_secs(30))
        .expect("desktop resize regression did not complete")
        .expect("desktop resize simulation failed");
    worker.join().expect("resize worker did not panic");
    run_result.expect("desktop event loop failed");
    assert_report(&explicit_report);
    assert_report(&content_report);
}

fn main() {
    if std::env::var_os("INCULAR_DESKTOP_LIVE_TESTS").is_none() {
        eprintln!(
            "in_place_resize: skipped live native regression; set INCULAR_DESKTOP_LIVE_TESTS=1 to run"
        );
        return;
    }
    programmatic_and_content_resizes_update_capture_surfaces_in_place();
}
