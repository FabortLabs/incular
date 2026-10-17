//! Target overlays: hover highlight, geometry, repaint/rebuild flashes, the
//! in-app performance graph and animation speed.
//!
//! One builder serves both the Widget inspector's Overlays tab and the
//! header's Overlays menu, so the toggles behave identically from any view.

use super::shared::{
    check_chip, column, gap, heading, refresh, segmented_control, text_muted, text_primary,
    ui_text, warning,
};
use crate::{inspector::Shared, transport::ClientBridge};
use incular::prelude::*;
use incular_devtools_protocol::{DebugOption, RequestMethod};

/// Protocol version that introduced [`DebugOption::PerformanceOverlay`].
pub const PERFORMANCE_OVERLAY_PROTOCOL: u32 = 4;

/// Animation speeds the target honours (it clamps the scale to `0..=4`).
pub const ANIMATION_SPEEDS: [(f32, &str); 7] = [
    (0., "Paused"),
    (0.1, "0.1×"),
    (0.25, "0.25×"),
    (0.5, "0.5×"),
    (1., "1×"),
    (2., "2×"),
    (4., "4×"),
];

/// One titled group of overlay toggles.
pub struct OverlayGroup {
    pub title: &'static str,
    pub description: &'static str,
    pub options: &'static [(DebugOption, &'static str)],
}

/// The option groups in display order (hover highlight and animation speed
/// are separate controls).
pub const GROUPS: [OverlayGroup; 4] = [
    OverlayGroup {
        title: "Layout",
        description: "Draw geometry on the selected widget, its subtree or every widget.",
        options: &[
            (DebugOption::LayoutBounds, "Bounds: selected"),
            (DebugOption::LayoutBoundsSubtree, "Bounds: subtree"),
            (DebugOption::LayoutBoundsWholeWindow, "Bounds: whole window"),
            (DebugOption::PaddingContent, "Padding / content"),
            (DebugOption::Baselines, "Baselines"),
            (DebugOption::Clips, "Clips"),
            (DebugOption::ScrollViewports, "Scroll viewports"),
        ],
    },
    OverlayGroup {
        title: "Hit test & semantics",
        description: "Show where pointers land and what assistive technology can reach.",
        options: &[
            (DebugOption::HitTestRegions, "Hit test regions"),
            (DebugOption::SemanticsBounds, "Semantics bounds"),
        ],
    },
    OverlayGroup {
        title: "Repaint & rebuild",
        description: "Flash widgets that did work in the last frame; the rainbow outlines every repaint in a new colour.",
        options: &[
            (DebugOption::HighlightBuild, "Build flash"),
            (DebugOption::HighlightLayout, "Layout flash"),
            (DebugOption::HighlightPaint, "Paint flash"),
            (DebugOption::HighlightSemantics, "Semantics flash"),
            (DebugOption::HighlightComposite, "Composite flash"),
            (DebugOption::RepaintRainbow, "Repaint rainbow"),
            (DebugOption::LayerBoundaries, "Layer boundaries"),
        ],
    },
    OverlayGroup {
        title: "Performance",
        description: "A frame-time graph drawn inside the app window (UI and raster bars against the frame budget).",
        options: &[(DebugOption::PerformanceOverlay, "Performance overlay")],
    },
];

pub(crate) struct OverlayState {
    pub(crate) connected: bool,
    pub(crate) protocol: u32,
    pub(crate) options: std::collections::HashSet<DebugOption>,
    pub(crate) animation_scale: f32,
    pub(crate) hover_highlight: bool,
}

impl OverlayState {
    pub(crate) fn read(shared: &Shared) -> Self {
        let state = shared.lock().expect("inspector state");
        Self {
            connected: state.connected,
            protocol: state
                .target_protocol
                .or(state.target_info.as_ref().map(|info| info.protocol_version))
                .unwrap_or(0),
            options: state.debug_options.clone(),
            animation_scale: state.animation_scale.unwrap_or(1.),
            hover_highlight: state.hover_highlight_enabled(),
        }
    }
}

/// Flips one debug option locally and on the target.
pub(crate) fn toggle_option(
    shared: &Shared,
    bridge: &ClientBridge,
    tick: &Signal<u64>,
    option: DebugOption,
) {
    let enabled = {
        let mut state = shared.lock().expect("inspector state");
        if state.debug_options.remove(&option) {
            false
        } else {
            state.debug_options.insert(option);
            true
        }
    };
    bridge.send_or_report(RequestMethod::SetDebugOption {
        name: option,
        enabled,
    });
    refresh(tick);
}

fn group_header(title: &str, description: &str) -> Widget {
    column([
        heading(title, 12.),
        gap(1., 3.),
        ui_text(description, 11., text_muted()),
        gap(1., 8.),
    ])
}

/// Every overlay control, grouped. `compact` narrows paddings for the menu.
pub(crate) fn overlays_panel(
    shared: &Shared,
    bridge: &ClientBridge,
    tick: &Signal<u64>,
    compact: bool,
) -> Widget {
    let state = OverlayState::read(shared);
    let mut body: Vec<Widget> = Vec::new();
    let space = if compact { 12. } else { 20. };
    if !state.connected {
        body.push(ui_text(
            "Connect to an app to use its overlays.",
            12.,
            warning(),
        ));
        body.push(gap(1., space));
    }

    body.push(group_header(
        "Hover highlight",
        "Outline a widget in the app while you hover its row. Selecting never leaves an overlay.",
    ));
    body.push({
        let (shared, bridge, tick) = (shared.clone(), bridge.clone(), tick.clone());
        let on = state.hover_highlight;
        check_chip("Highlight on hover", on, move || {
            super::hover::set_enabled(&shared, &bridge, &tick, !on);
        })
    });

    for group in &GROUPS {
        body.push(gap(1., space));
        body.push(group_header(group.title, group.description));
        if group.title == "Performance" && state.protocol < PERFORMANCE_OVERLAY_PROTOCOL {
            body.push(ui_text(
                "Requires a target built with DevTools protocol 4 or newer.",
                11.,
                warning(),
            ));
            continue;
        }
        let mut rows = Vec::new();
        for &(option, label) in group.options {
            let (shared, bridge, tick) = (shared.clone(), bridge.clone(), tick.clone());
            rows.push(check_chip(
                label,
                state.options.contains(&option),
                move || {
                    toggle_option(&shared, &bridge, &tick, option);
                },
            ));
        }
        body.push(Wrap::new(rows).spacing(4.).run_spacing(2.).into());
    }

    body.push(gap(1., space));
    body.push(group_header(
        "Animation speed",
        "Slow time down to inspect transitions, or fast-forward long ones; Paused freezes animations.",
    ));
    let speeds = ANIMATION_SPEEDS.map(|(scale, label)| {
        let (shared, bridge, tick) = (shared.clone(), bridge.clone(), tick.clone());
        let callback: Box<dyn Fn()> = Box::new(move || {
            // Reflect the choice at once: the target's confirmation can be
            // delayed or lost, and the control must never lag the click.
            if let Ok(mut state) = shared.lock() {
                state.animation_scale = Some(scale);
            }
            bridge.send_or_report(RequestMethod::SetAnimationSpeed { scale });
            refresh(&tick);
        });
        (
            label.to_owned(),
            (scale - state.animation_scale).abs() < 0.001,
            callback,
        )
    });
    body.push(segmented_control(speeds));
    body.push(ui_text(
        if state.animation_scale == 1. {
            "Running at normal speed".to_owned()
        } else if state.animation_scale == 0. {
            "Animations are paused".to_owned()
        } else {
            format!("Animations run at {}×", state.animation_scale)
        },
        11.,
        text_primary(),
    ));
    column(body)
}
