//! Opt-in native smoke review using the framework's own input and frame capture.
use incular::{prelude::*, testing::Simulation};
use std::path::Path;

fn capture(
    simulation: &Simulation,
    directory: &Path,
    name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    simulation.move_mouse_to(Offset::new(-1., -1.))?;
    simulation.wait_for_frame()?;
    let screenshot = simulation.capture()?;
    let path = directory.join(format!("{name}.png"));
    image::save_buffer(
        &path,
        screenshot.pixels(),
        screenshot.width(),
        screenshot.height(),
        image::ColorType::Rgba8,
    )?;
    println!(
        "Captured {}x{}: {}",
        screenshot.width(),
        screenshot.height(),
        path.display()
    );
    Ok(())
}

fn review(simulation: &Simulation, directory: &Path) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all(directory)?;
    for _ in 0..4 {
        simulation.wait_for_frame()?;
    }
    if std::env::var_os("INCULAR_DEVTOOLS_REVIEW_OFFLINE").is_some() {
        capture(simulation, directory, "offline")?;
        return Ok(());
    }
    simulation.click("Overview")?;
    capture(simulation, directory, "overview")?;
    simulation.click("Widget inspector")?;
    capture(simulation, directory, "widgets")?;
    if std::env::var_os("INCULAR_DEVTOOLS_REVIEW_INSPECTOR").is_some() {
        simulation.click("Expand tree")?;
        capture(simulation, directory, "inspector-large-tree")?;
        simulation.click("Inspect Row")?;
        simulation.press(Code::ArrowDown)?;
        simulation.press(Code::ArrowRight)?;
        capture(simulation, directory, "inspector-keyboard")?;
        simulation.key_down(Code::ControlLeft)?;
        simulation.key_down(Code::ShiftLeft)?;
        simulation.press(Code::KeyC)?;
        simulation.key_up(Code::ShiftLeft)?;
        simulation.key_up(Code::ControlLeft)?;
        capture(simulation, directory, "inspector-picking")?;
        simulation.press(Code::Escape)?;
        simulation.click("Focus subtree")?;
        capture(simulation, directory, "inspector-subtree")?;
        simulation.click("Whole tree")?;
        simulation.click("Layout")?;
        capture(simulation, directory, "inspector-layout")?;
        simulation.click("Overlays")?;
        capture(simulation, directory, "inspector-overlays")?;
        simulation.click("Properties")?;
        simulation.click("Identity & position")?;
        capture(simulation, directory, "inspector-groups")?;
        simulation.click_at(Offset::new(180., 174.))?;
        simulation.type_text("type:Text")?;
        simulation.press(Code::Enter)?;
        capture(simulation, directory, "inspector-search")?;
        simulation.press(Code::ArrowDown)?;
        capture(simulation, directory, "inspector-next-match")?;
        simulation.click("Reveal selection")?;
        capture(simulation, directory, "inspector-revealed")?;
        let width = std::env::var("INCULAR_DEVTOOLS_REVIEW_WIDTH")
            .ok()
            .and_then(|value| value.parse::<f32>().ok())
            .unwrap_or(1440.);
        let divider = (width * 0.59).clamp(280., width - 350.);
        simulation.mouse().down(Offset::new(divider, 500.))?;
        simulation
            .mouse()
            .move_to(Offset::new(divider - 70., 500.))?;
        simulation.mouse().up(Offset::new(divider - 70., 500.))?;
        capture(simulation, directory, "inspector-resized")?;
        simulation.click("Transport")?;
        capture(simulation, directory, "inspector-request-log")?;
        simulation.click("Export report")?;
        capture(simulation, directory, "inspector-report")?;
        let report = read_exported_report()?;
        if report["selected_widget"]["type_name"] != "Text" {
            return Err("Search/reveal did not retain the selected Text node's details".into());
        }
        if report["transport"]
            .as_array()
            .unwrap()
            .iter()
            .any(|request| {
                matches!(
                    request["status"].as_str(),
                    Some("failed" | "timed_out" | "disconnected")
                )
            })
        {
            return Err(
                "An inspector request failed; see the exported transport diagnostics".into(),
            );
        }
        return Ok(());
    }
    simulation.click("Layout")?;
    capture(simulation, directory, "widget-layout")?;
    simulation.click("Properties")?;
    simulation.click("Expand tree")?;
    capture(simulation, directory, "widgets-expanded")?;
    simulation.click("Overlays")?;
    capture(simulation, directory, "widget-visual-tools")?;
    simulation.click("Memory")?;
    simulation.click("Capture snapshot")?;
    for _ in 0..3 {
        simulation.wait_for_frame()?;
    }
    capture(simulation, directory, "memory")?;
    simulation.click("Set baseline A")?;
    simulation.click("Capture comparison B")?;
    for _ in 0..3 {
        simulation.wait_for_frame()?;
    }
    capture(simulation, directory, "memory-comparison")?;
    simulation.click("Console")?;
    capture(simulation, directory, "console")?;
    simulation.click("Pause display")?;
    capture(simulation, directory, "console-paused")?;
    simulation.click("Resume live")?;
    simulation.click("Errors")?;
    capture(simulation, directory, "console-filtered")?;
    simulation.click("Transport")?;
    capture(simulation, directory, "transport")?;
    simulation.click("Show failures only")?;
    capture(simulation, directory, "transport-filtered")?;
    simulation.click("Performance")?;
    capture(simulation, directory, "performance")?;
    simulation.click("Deep")?;
    simulation.click("Start recording")?;
    simulation.click("Widget inspector")?;
    simulation.click("Overlays")?;
    capture(simulation, directory, "widget-overlay-controls")?;
    simulation.click("Bounds: selected")?;
    simulation.click("Performance")?;
    for _ in 0..4 {
        simulation.wait_for_frame()?;
    }
    capture(simulation, directory, "performance-recording")?;
    simulation.click("Stop recording")?;
    simulation.click("Application")?;
    capture(simulation, directory, "application")?;
    simulation.click("Overview")?;
    capture(simulation, directory, "overview-captured")?;
    simulation.click("Export report")?;
    capture(simulation, directory, "report-exported")?;
    read_exported_report()?;
    Ok(())
}

fn read_exported_report() -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let reports = directories::UserDirs::new()
        .and_then(|dirs| dirs.download_dir().map(Path::to_owned))
        .unwrap_or_else(std::env::temp_dir)
        .join("Incular DevTools");
    let suffix = format!("-{}.json", std::process::id());
    let path = std::fs::read_dir(reports)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .find(|path| path.to_string_lossy().ends_with(&suffix))
        .ok_or("Export report did not create a JSON file")?;
    let report: serde_json::Value = serde_json::from_reader(std::fs::File::open(&path)?)?;
    if report["format"] != "incular-devtools-report"
        || !report["transport"].is_array()
        || !report["console"].is_array()
    {
        return Err("Exported report is missing diagnostics".into());
    }
    println!("Verified exported diagnostics: {}", path.display());
    Ok(report)
}

fn main() {
    let target_pid = incular_devtools_ui::session::requested_target_pid(std::env::args_os());
    let width = std::env::var("INCULAR_DEVTOOLS_REVIEW_WIDTH")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(1440.);
    let height = std::env::var("INCULAR_DEVTOOLS_REVIEW_HEIGHT")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(900.);
    let application = incular_devtools_ui::create_application_with_options(
        target_pid,
        WindowOptions {
            title: "Incular DevTools review".into(),
            initial_logical_size: Size::new(width, height),
            ..Default::default()
        },
    )
    .expect("DevTools application");
    let simulation = application.simulation();
    let directory = std::env::var_os("INCULAR_DEVTOOLS_REVIEW_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| "target/devtools-review".into());
    std::thread::spawn(move || match review(&simulation, &directory) {
        Ok(()) => {
            println!("Native DevTools smoke review passed");
            std::process::exit(0);
        }
        Err(error) => {
            eprintln!("Native DevTools review failed: {error}");
            std::process::exit(1);
        }
    });
    incular::run(application).expect("native DevTools host");
}
