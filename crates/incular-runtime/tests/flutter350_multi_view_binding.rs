//! Independent native window ownership, metrics, paint and input routing.

use std::{cell::RefCell, rc::Rc, time::Duration};

use incular_accessibility::{MobileAccessibilityProjection, SemanticActionRequest};
use incular_config::Constraints;
use incular_core::{Color, InputEvent, Offset, PointerPhase, Size, time::Instant};
use incular_platform::{
    PhysicalSize, PlatformEvent, WindowEvent, WindowId, WindowMetrics, WindowOptions,
};
use incular_rendering::PaintCommand;
use incular_runtime::Application;
use incular_semantics::{SemanticAction, SemanticNodeId};
use incular_widgets::{GestureDetector, Semantics, Widget};

const RED: Color = Color::rgba(255, 0, 0, 255);
const BLUE: Color = Color::rgba(0, 0, 255, 255);
const MISSING: WindowId = WindowId::from_parts(200, 1);

fn tile(color: Color) -> Widget {
    Widget::box_(Size::new(100., 100.), color)
}

fn app(first: Widget, second: Widget) -> (Application, WindowId, WindowId) {
    let mut app = Application::new(move |_| first.clone()).unwrap();
    let root = app.primary_window();
    let aux = app
        .open_window(WindowOptions::default(), second)
        .unwrap()
        .id();
    (app, root, aux)
}

fn metrics(app: &mut Application, id: WindowId, width: u32, height: u32, ratio: f64) {
    app.handle_window_event(WindowEvent::platform(
        id,
        PlatformEvent::Metrics(WindowMetrics::new(PhysicalSize::new(width, height), ratio)),
    ));
}

fn frame(app: &mut Application, id: WindowId) -> incular_rendering::DisplayList {
    let size = app.window_diagnostics(id).unwrap().logical_size;
    app.run_window_frame_at(id, Constraints::tight(size), Instant::now())
        .unwrap()
        .unwrap()
        .0
}

#[test]
fn registering_and_closing_views_changes_the_live_inventory_and_its_metrics() {
    let (mut app, first, second) = app(tile(RED), tile(BLUE));
    assert_eq!(app.active_window_ids(), [first, second]);
    metrics(&mut app, first, 400, 600, 2.5);
    let configured = app.window_diagnostics(first).unwrap();
    assert_eq!(configured.scale_factor, 2.5);
    assert_eq!(configured.logical_size, Size::new(160., 240.));

    assert!(app.close_window(second));
    assert_eq!(app.active_window_ids(), [first]);
    assert!(app.close_window(first));
    assert!(app.active_window_ids().is_empty());
}

#[test]
fn stale_or_unknown_view_ids_cannot_close_live_views() {
    let (mut app, first, second) = app(tile(RED), tile(BLUE));
    assert!(!app.close_window(MISSING));
    assert!(app.close_window(second));
    assert!(!app.close_window(second));
    let replacement = app
        .open_window(WindowOptions::default(), tile(BLUE))
        .unwrap()
        .id();
    assert_eq!(replacement.index(), second.index());
    assert_ne!(replacement.generation(), second.generation());
    assert!(!app.close_window(second));
    assert_eq!(app.active_window_ids(), [first, replacement]);
}

#[test]
fn host_metric_changes_reconfigure_only_the_target_view() {
    let (mut app, first, second) = app(tile(RED), tile(BLUE));
    metrics(&mut app, first, 400, 600, 2.5);
    metrics(&mut app, second, 420, 240, 2.);
    let original = app.window_diagnostics(first).unwrap();
    let untouched = app.window_diagnostics(second).unwrap();
    assert_eq!(original.logical_size, Size::new(160., 240.));

    metrics(&mut app, first, 300, 300, 3.);
    let updated = app.window_diagnostics(first).unwrap();
    assert_eq!(updated.scale_factor, 3.);
    assert_eq!(updated.logical_size, Size::new(100., 100.));
    assert_eq!(updated.surface_generation, original.surface_generation + 1);
    let paint = frame(&mut app, first);
    assert!(paint.commands().iter().any(|command| {
        matches!(command, PaintCommand::Rect { rect, color: RED } if rect.size == Size::new(100., 100.))
    }));
    let untouched_after = app.window_diagnostics(second).unwrap();
    assert_eq!(untouched.scale_factor, untouched_after.scale_factor);
    assert_eq!(untouched.logical_size, untouched_after.logical_size);
    assert_eq!(
        untouched.surface_generation,
        untouched_after.surface_generation
    );
}

fn first_semantic_node(app: &mut Application, view: WindowId, label: &str) -> SemanticNodeId {
    app.set_semantics_enabled(view, true);
    frame(app, view);
    app.sync_semantics_projection(view, &mut MobileAccessibilityProjection::new())
        .unwrap()
        .nodes
        .into_iter()
        .find(|node| node.semantic.label.as_deref() == Some(label))
        .expect("published semantic node")
        .semantic
        .id
}

#[test]
fn semantic_copy_and_tap_route_to_the_addressed_view_only() {
    let seen = Rc::new(RefCell::new(Vec::new()));
    let first: Widget = Semantics::new(tile(RED))
        .label("copy target")
        .container(true)
        .on_copy({
            let seen = seen.clone();
            move || seen.borrow_mut().push("copy")
        })
        .into();
    let second: Widget = Semantics::new(tile(BLUE))
        .label("tap target")
        .container(true)
        .on_tap({
            let seen = seen.clone();
            move || seen.borrow_mut().push("tap")
        })
        .into();
    let (mut app, first, second) = app(first, second);
    let copy = first_semantic_node(&mut app, first, "copy target");
    let tap = first_semantic_node(&mut app, second, "tap target");

    assert!(app.dispatch_accessibility_action(
        first,
        SemanticActionRequest {
            node: copy,
            action: SemanticAction::Copy
        }
    ));
    assert_eq!(*seen.borrow(), ["copy"]);
    assert!(app.dispatch_accessibility_action(
        second,
        SemanticActionRequest {
            node: tap,
            action: SemanticAction::Activate
        }
    ));
    assert_eq!(*seen.borrow(), ["copy", "tap"]);
    assert!(!app.dispatch_accessibility_action(
        MISSING,
        SemanticActionRequest {
            node: tap,
            action: SemanticAction::Activate
        }
    ));
    assert_eq!(*seen.borrow(), ["copy", "tap"]);
}

#[test]
fn frames_are_composited_for_live_window_roots_but_not_after_removal() {
    let (mut app, first, second) = app(tile(RED), tile(BLUE));
    assert_eq!(app.window_diagnostics(first).unwrap().presented_frames, 0);
    assert_eq!(app.window_diagnostics(second).unwrap().presented_frames, 0);
    let mut scenes = Vec::new();
    for round in 0..3 {
        for id in app.active_window_ids() {
            let paint = frame(&mut app, id);
            let expected = if id == first { RED } else { BLUE };
            assert!(paint.commands().iter().any(
                |command| matches!(command, PaintCommand::Rect { color, .. } if *color == expected)
            ));
            app.note_presented(id, true);
            scenes.push(id);
        }
        match round {
            0 => assert!(app.close_window(first)),
            1 => assert!(app.close_window(second)),
            _ => {}
        }
    }
    assert_eq!(scenes, [first, second, second]);
}

#[test]
fn identical_pointer_positions_hit_only_the_view_identified_in_the_input_packet() {
    let seen = Rc::new(RefCell::new(Vec::new()));
    let detector = |name, color| {
        let seen = seen.clone();
        Widget::from(GestureDetector::new(tile(color)).on_tap(move || seen.borrow_mut().push(name)))
    };
    let (mut app, first, second) = app(detector("first", RED), detector("second", BLUE));
    frame(&mut app, first);
    frame(&mut app, second);

    let now = Instant::now();
    for (n, (id, expected)) in [
        (first, &["first"][..]),
        (second, &["first", "second"][..]),
        (MISSING, &["first", "second"][..]),
    ]
    .into_iter()
    .enumerate()
    {
        for (offset, phase) in [PointerPhase::Down, PointerPhase::Up]
            .into_iter()
            .enumerate()
        {
            app.handle_window_event(WindowEvent::platform(
                id,
                PlatformEvent::InputAt {
                    input: InputEvent::PointerWithId {
                        pointer: 7,
                        phase,
                        position: Offset::new(20., 25.),
                    },
                    time: now + Duration::from_millis(100 * n as u64 + offset as u64),
                },
            ));
        }
        assert_eq!(&seen.borrow()[..], expected);
    }
}
