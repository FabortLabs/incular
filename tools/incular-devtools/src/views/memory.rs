use super::shared::{badge, column, key_value, metric, mono, page_title};
use super::{PRIMARY, TEXT_MUTED, TEXT_PRIMARY, compact_button, gap, section, ui_text};
use crate::{inspector::Shared, transport::ClientBridge};
use incular::prelude::*;
use incular_devtools_protocol::{RequestMethod, ResourceCounts};

fn comparison(first: &ResourceCounts, second: &ResourceCounts) -> Widget {
    let mut rows = vec![
        Row::new([
            Widget::from(Expanded::new(ui_text("RESOURCE", 10., TEXT_MUTED))),
            SizedBox::from_size(Size::new(90., 20.))
                .child(ui_text("BASELINE A", 10., TEXT_MUTED))
                .into(),
            SizedBox::from_size(Size::new(90., 20.))
                .child(ui_text("SNAPSHOT B", 10., TEXT_MUTED))
                .into(),
            SizedBox::from_size(Size::new(90., 20.))
                .child(ui_text("CHANGE", 10., TEXT_MUTED))
                .into(),
        ])
        .into(),
        gap(1., 10.),
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
        rows.push(
            Padding::new(
                EdgeInsets::symmetric(0., 9.),
                Row::new([
                    Widget::from(Expanded::new(ui_text(label, 12., TEXT_MUTED))),
                    SizedBox::from_size(Size::new(90., 20.))
                        .child(mono(a.to_string(), 12., TEXT_PRIMARY))
                        .into(),
                    SizedBox::from_size(Size::new(90., 20.))
                        .child(mono(b.to_string(), 12., TEXT_PRIMARY))
                        .into(),
                    SizedBox::from_size(Size::new(90., 20.))
                        .child(mono(
                            format!("{delta:+}"),
                            12.,
                            if delta > 0 {
                                super::shared::WARNING
                            } else if delta < 0 {
                                PRIMARY
                            } else {
                                TEXT_MUTED
                            },
                        ))
                        .into(),
                ]),
            )
            .into(),
        );
    }
    column(rows)
}

pub(crate) fn build_memory(shared: Shared, bridge: ClientBridge) -> Widget {
    let (memory, first, second) = shared
        .lock()
        .map(|state| {
            (
                state.memory.clone(),
                state.memory_a.clone(),
                state.memory_b.clone(),
            )
        })
        .unwrap_or_default();
    let capture = bridge.clone();
    let baseline = bridge.clone();
    let compare = bridge;
    let controls = Wrap::new([
        compact_button("Capture snapshot", false, move || {
            capture.send_or_report(RequestMethod::TakeMemorySnapshot {
                label: "manual".into(),
            })
        }),
        compact_button("Set baseline A", first.is_some(), move || {
            baseline.send_or_report(RequestMethod::TakeMemorySnapshot { label: "A".into() })
        }),
        compact_button("Capture comparison B", false, move || {
            compare.send_or_report(RequestMethod::TakeMemorySnapshot { label: "B".into() })
        }),
    ])
    .spacing(8.)
    .run_spacing(8.);
    let mut body = vec![
        page_title(
            "Memory",
            "Capture resource inventories and compare changes after an interaction.",
        ),
        controls.into(),
        gap(1., 20.),
    ];
    if let Some(memory) = memory {
        let counts = memory.counts;
        body.extend([
            Row::new([
                Expanded::new(metric(
                    "PROCESS RSS",
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
                    super::shared::VIOLET,
                ))
                .into(),
                gap(12., 1.),
                Expanded::new(metric(
                    "RETAINED ELEMENTS",
                    counts.elements.to_string(),
                    "Live framework elements",
                    TEXT_PRIMARY,
                ))
                .into(),
                gap(12., 1.),
                Expanded::new(metric(
                    "OFFSCREEN / EFFECT CACHE",
                    format!(
                        "{:.1} KiB",
                        (counts
                            .offscreen_bytes
                            .saturating_add(counts.effect_cached_bytes))
                            as f64
                            / 1024.
                    ),
                    "Reported renderer allocations",
                    PRIMARY,
                ))
                .into(),
            ])
            .into(),
            gap(1., 20.),
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
                gap(16., 1.),
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
            gap(1., 20.),
        ]);
    } else {
        body.extend([
            section(
                "Ready to take a snapshot",
                "Start with a baseline, interact with your application, then capture B.",
                ui_text(
                    "Capture a snapshot to see process memory and live framework resources.",
                    13.,
                    TEXT_MUTED,
                ),
            ),
            gap(1., 20.),
        ]);
    }
    let diff = match (first.as_ref(), second.as_ref()) {
        (Some(a), Some(b)) => comparison(&a.counts, &b.counts),
        _ => column([
            Row::new([
                badge(
                    if first.is_some() {
                        "A · captured"
                    } else {
                        "A · waiting"
                    },
                    if first.is_some() { PRIMARY } else { TEXT_MUTED },
                ),
                gap(12., 1.),
                badge(
                    if second.is_some() {
                        "B · captured"
                    } else {
                        "B · waiting"
                    },
                    if second.is_some() {
                        PRIMARY
                    } else {
                        TEXT_MUTED
                    },
                ),
            ])
            .into(),
            gap(1., 12.),
            ui_text(
                "Capture both snapshots to compare resource counts.",
                12.,
                TEXT_MUTED,
            ),
        ]),
    };
    body.push(section(
        "Snapshot comparison",
        "Inventory deltas identify growth; they do not attribute heap allocations or prove leaks.",
        diff,
    ));
    column(body)
}
