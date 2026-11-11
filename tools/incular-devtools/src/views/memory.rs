//! Memory: resource inventories, a session history of snapshots with trends,
//! and an A/B comparison of framework counts.

use super::console::session_time;
use super::icons::IconName;
use super::shared::{
    Variant, badge, button, caption, card, column, empty_state, gap, key_value, mono,
    primary_button, section, section_with, sparkline, stat_card, status_badge, surface_raised,
    wide_row,
};
use super::{primary, text_muted, text_primary, ui_text};
use crate::{
    inspector::{Shared, TimedSnapshot},
    transport::ClientBridge,
};
use incular::prelude::*;
use incular_devtools_protocol::{RequestMethod, ResourceCounts};

fn delta_color(delta: i128) -> Color {
    if delta > 0 {
        super::shared::warning()
    } else if delta < 0 {
        super::success()
    } else {
        text_muted()
    }
}

fn cell(width: f32, child: Widget) -> Widget {
    SizedBox::new().width(width).child(child).into()
}

fn comparison(first: &ResourceCounts, second: &ResourceCounts) -> Widget {
    let mut rows = vec![
        Container::with_child(Padding::new(
            EdgeInsets::symmetric(14., 9.),
            Row::new([
                Expanded::new(caption("Resource")).into(),
                cell(100., caption("Baseline A")),
                cell(100., caption("Snapshot B")),
                cell(90., caption("Change")),
            ]),
        ))
        .color(surface_raised())
        .radius(8.)
        .into(),
    ];
    for (label, a, b) in [
        ("Process RSS (MB)", first.rss_mb, second.rss_mb),
        ("Elements", first.elements, second.elements),
        (
            "Render objects",
            first.render_objects,
            second.render_objects,
        ),
        ("Layers", first.layers, second.layers),
        (
            "Semantics nodes",
            first.semantics_nodes,
            second.semantics_nodes,
        ),
        ("Signals", first.signals, second.signals),
        ("Active tasks", first.tasks_active, second.tasks_active),
        ("Images", first.image_resources, second.image_resources),
        (
            "Offscreen bytes",
            first.offscreen_bytes,
            second.offscreen_bytes,
        ),
        (
            "Effect cache bytes",
            first.effect_cached_bytes,
            second.effect_cached_bytes,
        ),
    ] {
        let delta = b as i128 - a as i128;
        let percent = if a > 0 {
            format!(" ({:+.0}%)", delta as f64 * 100. / a as f64)
        } else {
            String::new()
        };
        rows.push(
            Padding::new(
                EdgeInsets::symmetric(14., 8.),
                Row::new([
                    Expanded::new(ui_text(label, 12.5, text_muted())).into(),
                    cell(100., mono(a.to_string(), 12., text_primary())),
                    cell(100., mono(b.to_string(), 12., text_primary())),
                    cell(
                        90.,
                        mono(format!("{delta:+}{percent}"), 12., delta_color(delta)),
                    ),
                ])
                .cross_axis_alignment(CrossAxisAlignment::Center),
            )
            .into(),
        );
    }
    column(rows)
}

fn history_table(history: &[TimedSnapshot]) -> Widget {
    let mut rows = vec![
        Container::with_child(Padding::new(
            EdgeInsets::symmetric(14., 9.),
            Row::new([
                cell(110., caption("Taken")),
                cell(90., caption("Label")),
                Expanded::new(caption("RSS")).into(),
                cell(110., caption("Elements")),
                cell(90., caption("Layers")),
                cell(90., caption("Signals")),
            ]),
        ))
        .color(surface_raised())
        .radius(8.)
        .into(),
    ];
    for (index, entry) in history.iter().enumerate().rev() {
        let counts = &entry.snapshot.counts;
        let previous = index
            .checked_sub(1)
            .and_then(|previous| history.get(previous))
            .map(|entry| &entry.snapshot.counts);
        let with_delta = |value: usize, before: Option<usize>| -> Widget {
            let delta = before.map(|before| value as i128 - before as i128);
            let mut parts = vec![mono(value.to_string(), 12., text_primary())];
            if let Some(delta) = delta.filter(|delta| *delta != 0) {
                parts.push(gap(6., 1.));
                parts.push(mono(format!("{delta:+}"), 11., delta_color(delta)));
            }
            super::shared::hrow(parts)
        };
        rows.push(
            Padding::new(
                EdgeInsets::symmetric(14., 7.),
                Row::new([
                    cell(110., mono(session_time(entry.time_ms), 11.5, text_muted())),
                    cell(
                        90.,
                        Align::new(
                            Alignment::CENTER_LEFT,
                            badge(
                                entry.snapshot.label.clone(),
                                match entry.snapshot.label.as_str() {
                                    "A" | "B" => primary(),
                                    _ => text_muted(),
                                },
                            ),
                        )
                        .into(),
                    ),
                    Expanded::new(with_delta(
                        counts.rss_mb,
                        previous.map(|counts| counts.rss_mb),
                    ))
                    .into(),
                    cell(
                        110.,
                        with_delta(counts.elements, previous.map(|counts| counts.elements)),
                    ),
                    cell(
                        90.,
                        with_delta(counts.layers, previous.map(|counts| counts.layers)),
                    ),
                    cell(
                        90.,
                        with_delta(counts.signals, previous.map(|counts| counts.signals)),
                    ),
                ])
                .cross_axis_alignment(CrossAxisAlignment::Center),
            )
            .into(),
        );
    }
    column(rows)
}

fn trend(history: &[TimedSnapshot], value: fn(&ResourceCounts) -> usize, color: Color) -> Widget {
    let values = history
        .iter()
        .map(|entry| value(&entry.snapshot.counts) as f32)
        .collect::<Vec<_>>();
    LayoutBuilder::new(move |_, constraints| {
        sparkline(
            &values,
            constraints.max_width().clamp(40., 2000.),
            26.,
            color,
        )
    })
    .into()
}

pub(crate) fn build_memory(shared: Shared, bridge: ClientBridge) -> Widget {
    let (memory, first, second, history) = shared
        .lock()
        .map(|state| {
            (
                state.memory.clone(),
                state.memory_a.clone(),
                state.memory_b.clone(),
                state.memory_history().cloned().collect::<Vec<_>>(),
            )
        })
        .unwrap_or_default();
    let capture = bridge.clone();
    let baseline = bridge.clone();
    let compare = bridge;
    let toolbar = card(
        Padding::all(
            12.,
            wide_row([
                primary_button("Capture snapshot", Some(IconName::Chip), move || {
                    capture.send_or_report(RequestMethod::TakeMemorySnapshot {
                        label: "manual".into(),
                    })
                }),
                gap(8., 1.),
                button(
                    "Set baseline A",
                    None,
                    if first.is_some() {
                        Variant::Active
                    } else {
                        Variant::Secondary
                    },
                    true,
                    move || {
                        baseline
                            .send_or_report(RequestMethod::TakeMemorySnapshot { label: "A".into() })
                    },
                ),
                gap(8., 1.),
                button(
                    "Capture comparison B",
                    None,
                    if second.is_some() {
                        Variant::Active
                    } else {
                        Variant::Secondary
                    },
                    true,
                    move || {
                        compare
                            .send_or_report(RequestMethod::TakeMemorySnapshot { label: "B".into() })
                    },
                ),
                super::shared::spacer(),
                status_badge(
                    if first.is_some() {
                        "A captured"
                    } else {
                        "A waiting"
                    },
                    if first.is_some() {
                        super::success()
                    } else {
                        text_muted()
                    },
                ),
                gap(6., 1.),
                status_badge(
                    if second.is_some() {
                        "B captured"
                    } else {
                        "B waiting"
                    },
                    if second.is_some() {
                        super::success()
                    } else {
                        text_muted()
                    },
                ),
            ]),
        )
        .into(),
    );
    let mut body = vec![toolbar, gap(1., 14.)];
    if let Some(memory) = memory {
        let counts = memory.counts;
        body.extend([
            Row::new([
                Expanded::new(stat_card(
                    Some(IconName::Chip),
                    "Process RSS",
                    if counts.rss_mb == 0 {
                        "—".into()
                    } else {
                        format!("{} MB", counts.rss_mb)
                    },
                    if counts.rss_mb == 0 {
                        "Target did not report RSS"
                    } else {
                        "Resident process memory"
                    },
                    super::shared::violet(),
                    Some(trend(
                        &history,
                        |counts| counts.rss_mb,
                        super::shared::violet(),
                    )),
                ))
                .into(),
                gap(12., 1.),
                Expanded::new(stat_card(
                    Some(IconName::Tree),
                    "Retained elements",
                    counts.elements.to_string(),
                    "Live framework elements",
                    text_primary(),
                    Some(trend(&history, |counts| counts.elements, primary())),
                ))
                .into(),
                gap(12., 1.),
                Expanded::new(stat_card(
                    Some(IconName::Layers),
                    "Offscreen / effect cache",
                    format!(
                        "{:.1} KiB",
                        (counts
                            .offscreen_bytes
                            .saturating_add(counts.effect_cached_bytes))
                            as f64
                            / 1024.
                    ),
                    "Reported renderer allocations",
                    text_primary(),
                    Some(trend(
                        &history,
                        |counts| counts.offscreen_bytes + counts.effect_cached_bytes,
                        super::shared::info(),
                    )),
                ))
                .into(),
                gap(12., 1.),
                Expanded::new(stat_card(
                    Some(IconName::Layers),
                    "Compositor layers",
                    counts.layers.to_string(),
                    &format!("{} snapshots · latest {}", history.len(), memory.label),
                    text_primary(),
                    Some(trend(&history, |counts| counts.layers, super::success())),
                ))
                .into(),
            ])
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .into(),
            gap(1., 14.),
            Row::new([
                Expanded::new(section(
                    "Framework inventory",
                    format!("Snapshot: {}", memory.label),
                    column([
                        key_value("Render objects", counts.render_objects.to_string()),
                        key_value("Layers", counts.layers.to_string()),
                        key_value("Semantics nodes", counts.semantics_nodes.to_string()),
                        key_value("Signals", counts.signals.to_string()),
                        key_value("Active tasks", counts.tasks_active.to_string()),
                    ]),
                ))
                .into(),
                gap(14., 1.),
                Expanded::new(section(
                    "Renderer resources",
                    "Counts reported by the resource owner",
                    column([
                        key_value("Glyph atlas pages", counts.glyph_atlas_pages.to_string()),
                        key_value("Images", counts.image_resources.to_string()),
                        key_value("Gradients", counts.gradient_resources.to_string()),
                        key_value("Path meshes", counts.path_meshes.to_string()),
                        key_value("Effect cache", format!("{} B", counts.effect_cached_bytes)),
                    ]),
                ))
                .into(),
            ])
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .into(),
            gap(1., 14.),
            section_with(
                "Snapshot history",
                "Every snapshot this session, newest first, with the change from the one before",
                Some(badge(
                    format!(
                        "{} of {}",
                        history.len(),
                        crate::inspector::InspectorModel::MEMORY_HISTORY
                    ),
                    text_muted(),
                )),
                history_table(&history),
            ),
            gap(1., 14.),
        ]);
    } else {
        body.extend([
            card(empty_state(
                IconName::Chip,
                "Ready to take a snapshot",
                "Capture a snapshot to see process memory and live framework resources. \
                 Start with baseline A, interact with your application, then capture B.",
            )),
            gap(1., 14.),
        ]);
    }
    let diff = match (first.as_ref(), second.as_ref()) {
        (Some(a), Some(b)) => comparison(&a.counts, &b.counts),
        _ => ui_text(
            "Capture baseline A and comparison B to compare resource counts.",
            12.,
            text_muted(),
        ),
    };
    body.push(section(
        "Snapshot comparison",
        "Inventory deltas identify growth; they do not attribute heap allocations or prove leaks.",
        diff,
    ));
    column(body)
}
