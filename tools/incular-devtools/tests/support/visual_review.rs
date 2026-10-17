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

/// Clicks a control whose label appears only after an asynchronous target reply
/// (for example the Start/Stop recording toggle), retrying for a few seconds.
fn click_eventually(
    simulation: &Simulation,
    label: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut last = None;
    for _ in 0..120 {
        match simulation.click(label) {
            Ok(()) => return Ok(()),
            Err(error) => last = Some(error),
        }
        simulation.wait_for_frame()?;
    }
    Err(format!("never found a clickable '{label}': {last:?}").into())
}

fn review_performance(
    simulation: &Simulation,
    directory: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    simulation.click("Performance")?;
    capture(simulation, directory, "perf-empty")?;
    simulation.click("Timings")?;
    click_eventually(simulation, "Start recording")?;
    for _ in 0..90 {
        simulation.wait_for_frame()?;
    }
    click_eventually(simulation, "Stop recording")?;
    for _ in 0..4 {
        simulation.wait_for_frame()?;
    }
    capture(simulation, directory, "perf-basic")?;
    let bar = std::env::var("INCULAR_DEVTOOLS_REVIEW_BAR")
        .ok()
        .and_then(|value| {
            let (x, y) = value.split_once(',')?;
            Some(Offset::new(x.parse().ok()?, y.parse().ok()?))
        })
        .unwrap_or(Offset::new(1300., 600.));
    simulation.click_at(bar)?;
    capture(simulation, directory, "perf-selected")?;
    simulation.click("Deep")?;
    click_eventually(simulation, "Start recording")?;
    for _ in 0..60 {
        simulation.wait_for_frame()?;
    }
    click_eventually(simulation, "Stop recording")?;
    for _ in 0..4 {
        simulation.wait_for_frame()?;
    }
    capture(simulation, directory, "perf-deep")?;
    simulation.move_mouse_to(Offset::new(700., 500.))?;
    simulation.scroll(Offset::new(0., 700.))?;
    capture(simulation, directory, "perf-deep-scrolled")?;
    Ok(())
}

/// Waits (in real time, since the header text counts seconds) while the
/// transport settles into a state such as Busy or the offline target list.
fn settle(simulation: &Simulation, millis: u64) -> Result<(), Box<dyn std::error::Error>> {
    std::thread::sleep(std::time::Duration::from_millis(millis));
    for _ in 0..4 {
        simulation.wait_for_frame()?;
    }
    Ok(())
}

/// Clicks the first label that exists from a list of candidates.
fn click_first(simulation: &Simulation, labels: &[&str]) -> Result<(), Box<dyn std::error::Error>> {
    for _ in 0..40 {
        for label in labels {
            if simulation.click(*label).is_ok() {
                return Ok(());
            }
        }
        simulation.wait_for_frame()?;
    }
    Err(format!("none of {labels:?} became clickable").into())
}

/// Protocol v3 views: Rendering, top rebuilds, Semantics and Layers.
fn review_new(simulation: &Simulation, directory: &Path) -> Result<(), Box<dyn std::error::Error>> {
    click_eventually(simulation, "Refresh")?;
    simulation.click("Rendering")?;
    settle(simulation, 2500)?;
    capture(simulation, directory, "rendering")?;
    simulation.click("Reset deltas")?;
    simulation.wait_for_frame()?;
    simulation.move_mouse_to(Offset::new(700., 500.))?;
    simulation.scroll(Offset::new(0., 900.))?;
    capture(simulation, directory, "rendering-rebuilds")?;
    click_first(
        simulation,
        &[
            "Select Text in inspector",
            "Select Row in inspector",
            "Select Container in inspector",
            "Select DecoratedBox in inspector",
        ],
    )?;
    settle(simulation, 400)?;
    capture(simulation, directory, "rendering-to-inspector")?;
    simulation.click("Semantics")?;
    settle(simulation, 600)?;
    capture(simulation, directory, "semantics")?;
    click_first(
        simulation,
        &[
            "Select Button",
            "Select Text",
            "Select Label",
            "Select Window",
            "Select Group",
        ],
    )?;
    settle(simulation, 400)?;
    capture(simulation, directory, "semantics-selected")?;
    simulation.click("Layers")?;
    settle(simulation, 600)?;
    capture(simulation, directory, "layers")?;
    click_first(
        simulation,
        &[
            "Select Picture",
            "Select Transform",
            "Select Clip",
            "Select Root",
        ],
    )?;
    settle(simulation, 400)?;
    capture(simulation, directory, "layers-selected")?;
    simulation.click("Transport")?;
    capture(simulation, directory, "transport-new")?;
    Ok(())
}

/// Frame bar to click in the chart (window coordinates); override with
/// `INCULAR_DEVTOOLS_REVIEW_BAR=x,y`.
fn review_bar() -> Offset {
    std::env::var("INCULAR_DEVTOOLS_REVIEW_BAR")
        .ok()
        .and_then(|value| {
            let (x, y) = value.split_once(',')?;
            Some(Offset::new(x.parse().ok()?, y.parse().ok()?))
        })
        .unwrap_or(Offset::new(1300., 600.))
}

/// One pass over the main views, capturing `<prefix>-<view>.png`.
fn review_theme_pass(
    simulation: &Simulation,
    directory: &Path,
    prefix: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let name = |view: &str| format!("{prefix}-{view}");
    simulation.click("Widget inspector")?;
    settle(simulation, 400)?;
    capture(simulation, directory, &name("widgets"))?;
    simulation.click("Performance")?;
    simulation.click("Timings")?;
    click_eventually(simulation, "Start recording")?;
    for _ in 0..90 {
        simulation.wait_for_frame()?;
    }
    click_eventually(simulation, "Stop recording")?;
    for _ in 0..4 {
        simulation.wait_for_frame()?;
    }
    capture(simulation, directory, &name("performance"))?;
    simulation.click_at(review_bar())?;
    capture(simulation, directory, &name("performance-selected"))?;
    simulation.click("Rendering")?;
    settle(simulation, 2500)?;
    capture(simulation, directory, &name("rendering"))?;
    simulation.click("Semantics")?;
    settle(simulation, 600)?;
    click_first(
        simulation,
        &[
            "Select Button",
            "Select Text",
            "Select Label",
            "Select Window",
            "Select Group",
        ],
    )?;
    settle(simulation, 400)?;
    capture(simulation, directory, &name("semantics"))?;
    simulation.click("Layers")?;
    settle(simulation, 600)?;
    capture(simulation, directory, &name("layers"))?;
    simulation.click("Hide identity transforms")?;
    settle(simulation, 200)?;
    capture(simulation, directory, &name("layers-identity-shown"))?;
    simulation.click("Hide identity transforms")?;
    simulation.click("Memory")?;
    simulation.click("Capture snapshot")?;
    for _ in 0..3 {
        simulation.wait_for_frame()?;
    }
    capture(simulation, directory, &name("memory"))?;
    simulation.click("Console")?;
    capture(simulation, directory, &name("console"))?;
    simulation.click("Transport")?;
    capture(simulation, directory, &name("transport"))?;
    simulation.click("Application")?;
    capture(simulation, directory, &name("application"))?;
    Ok(())
}

/// Both themes: starts in `INCULAR_DEVTOOLS_THEME` (dark by default), then
/// uses the header toggle. With `INCULAR_DEVTOOLS_REVIEW_OFFLINE` it captures
/// the connection workspace in each theme instead.
fn review_themes(
    simulation: &Simulation,
    directory: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    use incular_devtools_ui::views::theme::{Theme, initial_theme};
    let offline = std::env::var_os("INCULAR_DEVTOOLS_REVIEW_OFFLINE").is_some();
    let mut theme = initial_theme();
    if offline {
        settle(simulation, 1500)?;
    }
    for _ in 0..2 {
        let prefix = match theme {
            Theme::Dark => "dark",
            Theme::Light => "light",
        };
        if offline {
            capture(simulation, directory, &format!("{prefix}-offline"))?;
        } else {
            review_theme_pass(simulation, directory, prefix)?;
        }
        simulation.click(theme.switch_label())?;
        theme = theme.toggled();
        settle(simulation, 300)?;
    }
    Ok(())
}

fn review(simulation: &Simulation, directory: &Path) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all(directory)?;
    for _ in 0..4 {
        simulation.wait_for_frame()?;
    }
    if std::env::var_os("INCULAR_DEVTOOLS_REVIEW_STRESS").is_some() {
        return review_stress(simulation, directory);
    }
    if std::env::var_os("INCULAR_DEVTOOLS_REVIEW_EDITORS").is_some() {
        return review_editors(simulation, directory);
    }
    if std::env::var_os("INCULAR_DEVTOOLS_REVIEW_ROUND2").is_some() {
        return review_round2(simulation, directory);
    }
    if std::env::var_os("INCULAR_DEVTOOLS_REVIEW_THEME").is_some() {
        return review_themes(simulation, directory);
    }
    if std::env::var_os("INCULAR_DEVTOOLS_REVIEW_OFFLINE").is_some() {
        let hold = std::env::var("INCULAR_DEVTOOLS_REVIEW_HOLD")
            .ok()
            .and_then(|value| value.parse::<u64>().ok())
            .unwrap_or(0);
        settle(simulation, 1500 + hold)?;
        capture(simulation, directory, "offline")?;
        if std::env::var_os("INCULAR_DEVTOOLS_REVIEW_TAKEOVER").is_some() {
            simulation.click("Widget inspector")?;
            click_eventually(simulation, "Take over")?;
            settle(simulation, 1500)?;
            capture(simulation, directory, "after-take-over")?;
        }
        return Ok(());
    }
    if std::env::var_os("INCULAR_DEVTOOLS_REVIEW_NEW").is_some() {
        review_new(simulation, directory)?;
        return Ok(());
    }
    if std::env::var_os("INCULAR_DEVTOOLS_REVIEW_PERFORMANCE").is_some() {
        review_performance(simulation, directory)?;
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
    click_eventually(simulation, "Start recording")?;
    simulation.click("Widget inspector")?;
    simulation.click("Overlays")?;
    capture(simulation, directory, "widget-overlay-controls")?;
    simulation.click("Bounds: selected")?;
    simulation.click("Performance")?;
    for _ in 0..4 {
        simulation.wait_for_frame()?;
    }
    capture(simulation, directory, "performance-recording")?;
    click_eventually(simulation, "Stop recording")?;
    simulation.click("Application")?;
    capture(simulation, directory, "application")?;
    simulation.click("Overview")?;
    capture(simulation, directory, "overview-captured")?;
    simulation.click("Export report")?;
    capture(simulation, directory, "report-exported")?;
    read_exported_report()?;
    Ok(())
}

/// Captures only the target application's window with the helper script, so
/// the review can prove what the *target* shows (for example that a hover
/// highlight is gone after the pointer leaves). The script lives in the
/// git-ignored scratch directory; without it the target capture is skipped.
fn capture_target(directory: &Path, name: &str) {
    let Some(pid) = incular_devtools_ui::session::requested_target_pid(std::env::args_os()) else {
        return;
    };
    let script = Path::new(".tmp/overlay-check/capture.ps1");
    if !script.exists() {
        println!(
            "Skipping target capture {name}: {} is missing",
            script.display()
        );
        return;
    }
    let Ok(output) = std::path::absolute(directory.join(format!("{name}.png"))) else {
        return;
    };
    let status = std::process::Command::new("powershell")
        .args(["-NoProfile", "-File"])
        .arg(script)
        .arg("-TargetPid")
        .arg(pid.to_string())
        .arg("-Out")
        .arg(&output)
        .status();
    println!("Target window capture {name}: {status:?}");
}

/// Like [`capture`] but leaves the pointer where it is, for hover states.
fn capture_hovering(
    simulation: &Simulation,
    directory: &Path,
    name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
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
    println!("Captured (hovering) {}", path.display());
    Ok(())
}

fn env_point(name: &str, default: Offset) -> Offset {
    std::env::var(name)
        .ok()
        .and_then(|value| {
            let (x, y) = value.split_once(',')?;
            Some(Offset::new(x.parse().ok()?, y.parse().ok()?))
        })
        .unwrap_or(default)
}

/// One theme's worth of round-2 captures: header with live FPS, hover
/// highlight (DevTools and target), the picker, overlays and animation speed.
fn review_round2_pass(
    simulation: &Simulation,
    directory: &Path,
    prefix: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let name = |view: &str| format!("{prefix}-r2-{view}");
    simulation.click("Widget inspector")?;
    simulation.click("Expand tree")?;
    settle(simulation, 1200)?;
    capture(simulation, directory, &name("header-live"))?;

    // Hover a tree row: the target should outline that widget and the row tint.
    let row = env_point("INCULAR_DEVTOOLS_REVIEW_ROW", Offset::new(200., 270.));
    simulation.move_mouse_to(row)?;
    settle(simulation, 500)?;
    capture_hovering(simulation, directory, &name("tree-hover"))?;
    capture_target(directory, &name("target-hovering"));
    // Leave: the overlay must be gone from the target.
    simulation.move_mouse_to(Offset::new(-1., -1.))?;
    settle(simulation, 500)?;
    capture_target(directory, &name("target-after-leave"));

    // Select a row, then leave: selection never keeps an overlay.
    simulation.click_at(row)?;
    simulation.move_mouse_to(Offset::new(-1., -1.))?;
    settle(simulation, 500)?;
    capture(simulation, directory, &name("selected"))?;
    capture_target(directory, &name("target-after-select"));

    // Picker: obvious state in the toolbar, header pill and footer.
    simulation.click("Pick widget")?;
    settle(simulation, 400)?;
    capture(simulation, directory, &name("picking"))?;
    capture_target(directory, &name("target-picking"));
    // Switching view must end the pick.
    simulation.click("Performance")?;
    settle(simulation, 400)?;
    capture(simulation, directory, &name("picker-ended-by-view-switch"))?;
    capture_target(directory, &name("target-after-view-switch"));
    simulation.click("Widget inspector")?;
    simulation.click("Pick widget")?;
    settle(simulation, 200)?;
    simulation.press(Code::Escape)?;
    settle(simulation, 400)?;
    capture(simulation, directory, &name("picker-ended-by-escape"))?;
    capture_target(directory, &name("target-after-escape"));

    // Overlays tab, animation speed, and the header menu.
    simulation.click("Overlays")?;
    settle(simulation, 300)?;
    capture(simulation, directory, &name("overlays"))?;
    simulation.move_mouse_to(Offset::new(1500., 700.))?;
    simulation.scroll(Offset::new(0., 2000.))?;
    settle(simulation, 300)?;
    capture_hovering(simulation, directory, &name("overlays-scrolled"))?;
    simulation.click("0.25×")?;
    settle(simulation, 400)?;
    capture(simulation, directory, &name("animation-speed"))?;
    simulation.click("0.5×")?;
    simulation.click("1×")?;
    simulation.click("Performance")?;
    simulation.click("Overlays menu")?;
    settle(simulation, 300)?;
    capture(simulation, directory, &name("overlays-menu"))?;
    simulation.press(Code::Escape)?;
    settle(simulation, 300)?;
    capture(simulation, directory, &name("overlays-menu-closed"))?;
    Ok(())
}

/// Round-2 review (`INCULAR_DEVTOOLS_REVIEW_ROUND2=1`): both themes.
fn review_round2(
    simulation: &Simulation,
    directory: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    use incular_devtools_ui::views::theme::{Theme, initial_theme};
    let mut theme = initial_theme();
    for _ in 0..2 {
        let prefix = match theme {
            Theme::Dark => "dark",
            Theme::Light => "light",
        };
        review_round2_pass(simulation, directory, prefix)?;
        simulation.click(theme.switch_label())?;
        theme = theme.toggled();
        settle(simulation, 300)?;
    }
    Ok(())
}

/// Selects the first widget matching `key:<key>` through the tree search.
fn select_by_key(simulation: &Simulation, key: &str) -> Result<(), Box<dyn std::error::Error>> {
    simulation.click_at(Offset::new(180., 174.))?;
    simulation.key_down(Code::ControlLeft)?;
    simulation.press(Code::KeyA)?;
    simulation.key_up(Code::ControlLeft)?;
    simulation.type_text(format!("key:{key}"))?;
    simulation.press(Code::Enter)?;
    settle(simulation, 500)
}

/// Replaces the text of the field labelled `label` and presses Enter.
fn type_into(
    simulation: &Simulation,
    label: &str,
    text: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    click_eventually(simulation, label)?;
    simulation.key_down(Code::ControlLeft)?;
    simulation.press(Code::KeyA)?;
    simulation.key_up(Code::ControlLeft)?;
    simulation.type_text(text)?;
    simulation.press(Code::Enter)?;
    settle(simulation, 400)
}

fn press_label(simulation: &Simulation, label: &str) -> Result<(), Box<dyn std::error::Error>> {
    click_eventually(simulation, label)?;
    settle(simulation, 350)
}

/// Captures the DevTools window and the target window under `name`.
fn capture_both(
    simulation: &Simulation,
    directory: &Path,
    name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    settle(simulation, 300)?;
    capture(simulation, directory, &format!("{name}-devtools"))?;
    capture_target(directory, &format!("{name}-target"));
    Ok(())
}

/// One theme's editor tour: every editor kind is operated against the
/// playground's "Editable widgets" page, with DevTools and target captures to
/// prove the edit reached the target.
fn review_editors_pass(
    simulation: &Simulation,
    directory: &Path,
    prefix: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let name = |step: &str| format!("{prefix}-ed-{step}");
    simulation.click("Widget inspector")?;
    simulation.click("Expand tree")?;
    settle(simulation, 800)?;

    // Text: stepper, typed colour, palette, string, revert.
    select_by_key(simulation, "edit-text")?;
    capture_both(simulation, directory, &name("text-initial"))?;
    for _ in 0..3 {
        press_label(simulation, "Increase fontSize")?;
    }
    type_into(simulation, "color hex", "#E53935")?;
    type_into(simulation, "text value", "Edited in DevTools")?;
    capture_both(simulation, directory, &name("text-edited"))?;
    press_label(simulation, "Color palette color")?;
    capture_both(simulation, directory, &name("text-palette"))?;
    press_label(simulation, "Swatch #4CAF50")?;
    capture_both(simulation, directory, &name("text-swatch"))?;
    press_label(simulation, "Revert fontSize")?;
    capture_both(simulation, directory, &name("text-reverted-size"))?;

    // Padding: insets with link-all.
    select_by_key(simulation, "edit-padding")?;
    capture_both(simulation, directory, &name("padding-initial"))?;
    press_label(simulation, "Link all padding")?;
    type_into(simulation, "padding L", "36")?;
    capture_both(simulation, directory, &name("padding-link-toggled"))?;
    press_label(simulation, "Link all padding")?;
    type_into(simulation, "padding T", "8")?;
    capture_both(simulation, directory, &name("padding-linked-edit"))?;

    // Decorated box: colour hex, radius and border steppers.
    select_by_key(simulation, "edit-decorated")?;
    capture_both(simulation, directory, &name("decorated-initial"))?;
    type_into(simulation, "background hex", "#26A69ACC")?;
    for _ in 0..4 {
        press_label(simulation, "Increase radius")?;
    }
    type_into(simulation, "borderWidth value", "4")?;
    type_into(simulation, "borderColor hex", "#3F51B5")?;
    capture_both(simulation, directory, &name("decorated-edited"))?;

    // Sized box and constrained box (unbounded maximum).
    select_by_key(simulation, "edit-sizedbox")?;
    capture_both(simulation, directory, &name("sizedbox-initial"))?;
    for _ in 0..3 {
        press_label(simulation, "Increase width")?;
    }
    type_into(simulation, "height value", "90")?;
    capture_both(simulation, directory, &name("sizedbox-edited"))?;
    select_by_key(simulation, "edit-constrained")?;
    capture_both(simulation, directory, &name("constrained-initial"))?;

    // Row: menu and segmented enums, then spacing.
    select_by_key(simulation, "edit-row")?;
    capture_both(simulation, directory, &name("row-initial"))?;
    press_label(simulation, "Choose mainAxisAlignment")?;
    capture_both(simulation, directory, &name("row-menu-open"))?;
    press_label(simulation, "mainAxisAlignment SpaceAround")?;
    press_label(simulation, "mainAxisSize Min")?;
    for _ in 0..3 {
        press_label(simulation, "Increase spacing")?;
    }
    capture_both(simulation, directory, &name("row-edited"))?;
    select_by_key(simulation, "edit-column")?;
    press_label(simulation, "Choose crossAxisAlignment")?;
    press_label(simulation, "crossAxisAlignment Center")?;
    capture_both(simulation, directory, &name("column-edited"))?;

    // Align: 3x3 grid.
    select_by_key(simulation, "edit-align")?;
    capture_both(simulation, directory, &name("align-initial"))?;
    press_label(simulation, "alignment TopLeft")?;
    capture_both(simulation, directory, &name("align-edited"))?;

    // Opacity: stepper and arrow keys inside the field.
    select_by_key(simulation, "edit-opacity")?;
    capture_both(simulation, directory, &name("opacity-initial"))?;
    press_label(simulation, "Decrease opacity")?;
    click_eventually(simulation, "opacity value")?;
    for _ in 0..4 {
        simulation.press(Code::ArrowDown)?;
    }
    settle(simulation, 500)?;
    capture_both(simulation, directory, &name("opacity-edited"))?;

    // Reset everything and prove the target went back.
    press_label(simulation, "Reset all overrides")?;
    settle(simulation, 700)?;
    capture_both(simulation, directory, &name("opacity-reset"))?;
    select_by_key(simulation, "edit-padding")?;
    capture_both(simulation, directory, &name("padding-reset"))?;
    Ok(())
}

/// Editor review (`INCULAR_DEVTOOLS_REVIEW_EDITORS=1`): both themes.
fn review_editors(
    simulation: &Simulation,
    directory: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    use incular_devtools_ui::views::theme::{Theme, initial_theme};
    let mut theme = initial_theme();
    for _ in 0..2 {
        let prefix = match theme {
            Theme::Dark => "dark",
            Theme::Light => "light",
        };
        review_editors_pass(simulation, directory, prefix)?;
        simulation.click(theme.switch_label())?;
        theme = theme.toggled();
        settle(simulation, 300)?;
    }
    Ok(())
}

/// Clicks around quickly (`INCULAR_DEVTOOLS_REVIEW_STRESS=1`): every view,
/// the divider drag, search, the picker, overlays and both themes, then proves
/// the UI still answers by using it. A wedged model lock, a stuck pointer
/// capture or a lingering barrier would make the final clicks fail.
fn review_stress(
    simulation: &Simulation,
    directory: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let views = [
        "Overview",
        "Widget inspector",
        "Performance",
        "Rendering",
        "Semantics",
        "Layers",
        "Memory",
        "Console",
        "Transport",
        "Application",
    ];
    for round in 0..3 {
        for view in views {
            simulation.click(view)?;
            simulation.wait_for_frame()?;
        }
        simulation.click("Widget inspector")?;
        let divider = Offset::new(0.59 * 1440., 500.);
        simulation.mouse().down(divider)?;
        simulation
            .mouse()
            .move_to(Offset::new(divider.x - 120., 500.))?;
        simulation.mouse().up(Offset::new(divider.x - 120., 500.))?;
        simulation.click_at(Offset::new(180., 174.))?;
        simulation.type_text("type:Text")?;
        simulation.press(Code::Enter)?;
        simulation.click("Pick widget")?;
        simulation.click("Performance")?;
        simulation.click("Widget inspector")?;
        simulation.click("Pick widget")?;
        simulation.click("Picking… (Esc)")?;
        simulation.click("Overlays menu")?;
        simulation.click("Performance")?;
        simulation.click("Overlays menu")?;
        simulation
            .click("Light theme")
            .or_else(|_| simulation.click("Dark theme"))?;
        println!("stress round {round} done");
    }
    capture(simulation, directory, "stress-end")?;
    simulation.click("Widget inspector")?;
    click_eventually(simulation, "Expand tree")?;
    click_eventually(simulation, "Pick widget")?;
    simulation.press(Code::Escape)?;
    click_eventually(simulation, "Pick widget")?;
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
