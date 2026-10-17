#![cfg(feature = "devtools")]

//! Select Widget capture rules and flash/performance overlay behavior of the
//! DevTools runner, driven against a real retained window.

use incular_config::Constraints;
use incular_core::{Color, Offset, Size};
use incular_desktop::devtools_overlay::FrameSample;
use incular_desktop::devtools_runner::{DevToolsState, InspectInput};
use incular_devtools_protocol::{DebugOption, DebugValue, DevWindowId, TreeDelta};
use incular_platform::WindowId;
use incular_rendering::DisplayList;
use incular_runtime::Application;
use incular_widgets::Widget;
use std::time::{Duration, Instant};

fn application() -> (Application, WindowId) {
    let application = Application::new(|_| {
        Widget::from(incular_widgets::Padding::all(
            10.,
            Widget::box_(Size::new(40., 40.), Color::WHITE),
        ))
    })
    .unwrap();
    let window = application.primary_window();
    (application, window)
}

fn frame(application: &mut Application, window: WindowId) {
    application
        .run_window_frame_at(
            window,
            Constraints::tight(Size::new(200., 120.)),
            Instant::now(),
        )
        .unwrap();
}

fn protocol_window(window: WindowId) -> DevWindowId {
    DevWindowId::new(
        u64::from(window.index()) + 1,
        u64::from(window.generation()),
    )
}

fn root_id(application: &Application, window: WindowId) -> incular_devtools_protocol::DevWidgetId {
    let snapshot = application
        .devtools_widget_tree(protocol_window(window))
        .unwrap();
    let TreeDelta::Snapshot { root, .. } = &snapshot[0] else {
        panic!("snapshot");
    };
    root.id
}

const PRESS: InspectInput = InspectInput {
    moved: false,
    pressed: true,
    released: false,
};
const RELEASE: InspectInput = InspectInput {
    moved: false,
    pressed: false,
    released: true,
};
const MOVE: InspectInput = InspectInput {
    moved: true,
    pressed: false,
    released: false,
};

#[test]
fn idle_state_never_captures_pointer_input() {
    let (application, window) = application();
    let mut state = DevToolsState::new(None);
    assert!(!state.captures_pointer(window));
    assert!(!state.inspect_pointer(&application, window, Offset::new(5., 5.), PRESS));
    assert!(!state.inspect_pointer(&application, window, Offset::new(5., 5.), RELEASE));
}

#[test]
fn press_over_empty_space_keeps_selecting_and_swallows_the_release() {
    let (mut application, window) = application();
    frame(&mut application, window);
    let mut state = DevToolsState::new(None);
    state.begin_inspection(window);
    // Far outside the 200x120 window: nothing to hit.
    let nowhere = Offset::new(9_000., 9_000.);
    assert!(state.inspect_pointer(&application, window, nowhere, PRESS));
    assert!(
        state.is_selecting(window),
        "an empty press must not end the mode"
    );
    assert!(state.inspect_pointer(&application, window, nowhere, RELEASE));
    assert!(state.is_selecting(window));
    assert!(state.captures_pointer(window));
}

#[test]
fn a_pick_ends_the_mode_but_captures_until_the_matching_release() {
    let (mut application, window) = application();
    frame(&mut application, window);
    let mut state = DevToolsState::new(None);
    state.begin_inspection(window);
    let inside = Offset::new(20., 20.);
    assert!(state.inspect_pointer(&application, window, inside, MOVE));
    assert!(state.is_selecting(window));
    assert!(state.inspect_pointer(&application, window, inside, PRESS));
    assert!(
        !state.is_selecting(window),
        "the pick ends Select Widget mode"
    );
    assert!(state.captures_pointer(window), "the gesture stays captured");
    // Drag moves of the consumed gesture never reach the app.
    assert!(state.inspect_pointer(&application, window, inside, MOVE));
    assert!(state.inspect_pointer(&application, window, inside, RELEASE));
    assert!(!state.captures_pointer(window));
    // The next gesture belongs to the application again.
    assert!(!state.inspect_pointer(&application, window, inside, PRESS));
    assert!(!state.inspect_pointer(&application, window, inside, RELEASE));
}

#[test]
fn cancelling_clears_a_pending_capture() {
    let (mut application, window) = application();
    frame(&mut application, window);
    let mut state = DevToolsState::new(None);
    state.begin_inspection(window);
    assert!(state.inspect_pointer(&application, window, Offset::new(20., 20.), PRESS));
    assert!(state.captures_pointer(window));
    state.cancel_inspection(window);
    assert!(!state.captures_pointer(window));
    state.begin_inspection(window);
    assert!(state.cancel_inspection(window));
    assert!(!state.captures_pointer(window));
}

#[test]
fn inspect_mode_draws_a_window_border_only_while_active() {
    let (_application, window) = application();
    let mut state = DevToolsState::new(None);
    let size = Size::new(200., 120.);
    let mut idle = DisplayList::new();
    state.paint_overlay(window, size, &mut idle);
    assert!(idle.is_empty());
    state.begin_inspection(window);
    let mut active = DisplayList::new();
    state.paint_overlay(window, size, &mut active);
    assert!(active.len() >= 8, "two stacked frames of four edges");
}

#[test]
fn flashes_fade_out_and_request_redraws_until_done() {
    let (mut application, window) = application();
    frame(&mut application, window);
    let mut state = DevToolsState::new(None);
    state.set_option(DebugOption::HighlightBuild, true);
    state.set_option(DebugOption::HighlightLayout, true);
    state.frame_update(&application, window);
    assert!(!state.wants_redraw(window), "nothing changed yet");

    let id = root_id(&application, window);
    assert!(application.devtools_edit_property(id, "padding", &DebugValue::Insets([30.; 4])));
    frame(&mut application, window);
    state.frame_update(&application, window);
    assert!(state.wants_redraw(window), "a fresh flash keeps redrawing");

    let size = Size::new(200., 120.);
    let mut fresh = DisplayList::new();
    state.paint_overlay(window, size, &mut fresh);
    assert!(!fresh.is_empty(), "the flash is painted");

    std::thread::sleep(Duration::from_millis(560));
    assert!(!state.wants_redraw(window), "the fade finished");
    let mut faded = DisplayList::new();
    state.paint_overlay(window, size, &mut faded);
    assert!(faded.is_empty(), "expired flashes draw nothing");
    state.frame_update(&application, window);
    assert!(!state.wants_redraw(window));
}

#[test]
fn repaint_rainbow_outlines_persist_and_rotate() {
    let (mut application, window) = application();
    frame(&mut application, window);
    let mut state = DevToolsState::new(None);
    state.set_option(DebugOption::RepaintRainbow, true);
    state.frame_update(&application, window);
    let id = root_id(&application, window);
    let size = Size::new(200., 120.);
    let mut commands = Vec::new();
    for edge in [12., 14., 16.] {
        assert!(application.devtools_edit_property(id, "padding", &DebugValue::Insets([edge; 4])));
        frame(&mut application, window);
        state.frame_update(&application, window);
        let mut list = DisplayList::new();
        state.paint_overlay(window, size, &mut list);
        commands.push(format!("{:?}", list.commands()));
        // Persistent: no redraw is requested for the rainbow itself.
        assert!(!state.wants_redraw(window));
    }
    assert!(commands.iter().all(|list| list.len() > 2));
    assert_ne!(commands[0], commands[1], "colour rotates per repaint");
    assert_ne!(commands[1], commands[2]);
    // Still drawn long after the repaint, until the option is turned off.
    std::thread::sleep(Duration::from_millis(560));
    let mut later = DisplayList::new();
    state.paint_overlay(window, size, &mut later);
    assert_eq!(format!("{:?}", later.commands()), commands[2]);
    state.set_option(DebugOption::RepaintRainbow, false);
    state.frame_update(&application, window);
    let mut off = DisplayList::new();
    state.paint_overlay(window, size, &mut off);
    assert!(off.is_empty());
}

#[test]
fn performance_overlay_draws_recent_frames_only_when_enabled() {
    let (_application, window) = application();
    let mut state = DevToolsState::new(None);
    let size = Size::new(400., 300.);
    for index in 0..300_u32 {
        state.record_frame_sample(
            window,
            FrameSample {
                cpu_us: 2_000 + index * 10,
                gpu_us: Some(1_000),
                budget_us: Some(16_667),
            },
        );
    }
    let mut off = DisplayList::new();
    state.paint_overlay(window, size, &mut off);
    assert!(off.is_empty());
    state.set_option(DebugOption::PerformanceOverlay, true);
    let mut on = DisplayList::new();
    state.paint_overlay(window, size, &mut on);
    // Bounded by the 120-frame window: two rows of bars plus chrome and labels.
    assert!(on.len() > 240 && on.len() < 400, "{}", on.len());
}
