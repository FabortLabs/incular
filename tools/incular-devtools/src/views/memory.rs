//! Memory: process totals and what holds them, resource inventories, a
//! session history of snapshots with trends, and an A/B comparison.

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
    memory::{
        MemoryGroup, bucket_changes, format_byte_delta, format_bytes, memory_groups,
        unattributed_bytes,
    },
    transport::ClientBridge,
};
use incular::prelude::*;
use incular_devtools_protocol::{MemorySnapshot, ProcessMemory, RequestMethod, ResourceCounts};

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

/// Rows a breakdown table shows before folding the rest into one line.
const BUCKET_ROWS: usize = 14;

fn table_header(cells: Vec<Widget>) -> Widget {
    Container::with_child(Padding::new(
        EdgeInsets::symmetric(14., 9.),
        Row::new(cells),
    ))
    .color(surface_raised())
    .radius(8.)
    .into()
}

/// A bar whose length is `fraction` of the column it sits in.
fn share_bar(fraction: f32, color: Color) -> Widget {
    let width = (150. * fraction.clamp(0., 1.)).max(2.);
    Align::new(
        Alignment::CENTER_LEFT,
        Container::with_child(gap(width, 6.))
            .color(color)
            .radius(3.),
    )
    .into()
}

fn process_section(process: &ProcessMemory) -> Widget {
    if *process == ProcessMemory::default() {
        return section(
            "Process memory",
            "What the operating system charges this process",
            ui_text(
                "The target did not report process memory. Update it to an Incular build \
                 that reports memory breakdowns.",
                12.,
                text_muted(),
            ),
        );
    }
    let heap = match process.heap_live_bytes {
        Some(live) => format!(
            "{} live · peak {} · {} allocations",
            format_bytes(live),
            format_bytes(process.heap_peak_bytes.unwrap_or(live)),
            process.heap_live_allocations.unwrap_or(0),
        ),
        None => "Not tracked: install incular::core::memory::CountingAllocator as the \
                 global allocator"
            .into(),
    };
    let gpu = match (process.gpu_allocated_bytes, process.gpu_reserved_bytes) {
        (Some(allocated), Some(reserved)) => format!(
            "{} in use · {} reserved in {} blocks",
            format_bytes(allocated),
            format_bytes(reserved),
            process.gpu_blocks,
        ),
        _ => "Not reported by this graphics backend".into(),
    };
    let other = unattributed_bytes(process).map_or_else(
        || "Unknown without a heap or GPU figure".to_owned(),
        |bytes| {
            format!(
                "{} · native libraries, graphics driver, stacks",
                format_bytes(bytes)
            )
        },
    );
    section(
        "Process memory",
        "What the operating system charges this process, and how much of it the Rust heap and \
         the GPU allocator explain",
        column([
            key_value("Private", format_bytes(process.private_bytes)),
            key_value(
                "Resident",
                format!(
                    "{} · peak {}",
                    format_bytes(process.resident_bytes),
                    format_bytes(process.peak_resident_bytes)
                ),
            ),
            key_value("Rust heap", heap),
            key_value("GPU memory", gpu),
            key_value("Unattributed", other),
        ]),
    )
}

fn group_section(group: &MemoryGroup, color: Color) -> Widget {
    let mut rows = vec![table_header(vec![
        Expanded::new(caption("Owner")).into(),
        cell(70., caption("Count")),
        cell(100., caption("Size")),
        cell(160., caption("Share")),
    ])];
    let total = group.total.max(1) as f32;
    for bucket in group.buckets.iter().take(BUCKET_ROWS) {
        rows.push(
            Padding::new(
                EdgeInsets::symmetric(14., 7.),
                Row::new([
                    Expanded::new(ui_text(bucket.name.clone(), 12.5, text_primary())).into(),
                    cell(70., mono(bucket.count.to_string(), 12., text_muted())),
                    cell(100., mono(format_bytes(bucket.bytes), 12., text_primary())),
                    cell(160., share_bar(bucket.bytes as f32 / total, color)),
                ])
                .cross_axis_alignment(CrossAxisAlignment::Center),
            )
            .into(),
        );
    }
    if group.buckets.len() > BUCKET_ROWS {
        let rest = &group.buckets[BUCKET_ROWS..];
        let bytes = rest.iter().map(|bucket| bucket.bytes).sum::<u64>();
        rows.push(
            Padding::new(
                EdgeInsets::symmetric(14., 7.),
                ui_text(
                    format!("{} smaller owners · {}", rest.len(), format_bytes(bytes)),
                    12.,
                    text_muted(),
                ),
            )
            .into(),
        );
    }
    section_with(
        &group.name,
        group_description(&group.name),
        Some(badge(format_bytes(group.total), color)),
        column(rows),
    )
}

fn group_description(group: &str) -> &'static str {
    use incular_devtools_protocol::memory_groups;
    match group {
        memory_groups::GPU_ALLOCATIONS => {
            "Live GPU allocations by resource label, as the backend allocator sees them"
        }
        memory_groups::RENDERER => {
            "What each renderer cache retains; the same GPU memory seen by owner"
        }
        memory_groups::LAYERS => "The largest retained layer textures and how long each has idled",
        memory_groups::CPU => "Process heap owners the framework can attribute",
        _ => "Reported by the target",
    }
}

/// Process totals and one table per bucket group of `snapshot`.
fn memory_breakdown(snapshot: &MemorySnapshot) -> Vec<Widget> {
    let mut widgets = vec![process_section(&snapshot.process), gap(1., 14.)];
    let colors = [
        super::shared::violet(),
        super::shared::info(),
        primary(),
        super::success(),
    ];
    for (index, group) in memory_groups(snapshot).iter().enumerate() {
        widgets.push(group_section(group, colors[index % colors.len()]));
        widgets.push(gap(1., 14.));
    }
    widgets
}

fn byte_row(label: String, before: u64, after: u64) -> Widget {
    let delta = i128::from(after) - i128::from(before);
    Padding::new(
        EdgeInsets::symmetric(14., 8.),
        Row::new([
            Expanded::new(ui_text(label, 12.5, text_muted())).into(),
            cell(100., mono(format_bytes(before), 12., text_primary())),
            cell(100., mono(format_bytes(after), 12., text_primary())),
            cell(
                90.,
                mono(
                    if delta == 0 {
                        "0".to_owned()
                    } else {
                        format_byte_delta(delta)
                    },
                    12.,
                    delta_color(delta),
                ),
            ),
        ])
        .cross_axis_alignment(CrossAxisAlignment::Center),
    )
    .into()
}

/// Process bytes and the owners that changed most between two snapshots.
fn byte_comparison(first: &MemorySnapshot, second: &MemorySnapshot) -> Widget {
    let (a, b) = (&first.process, &second.process);
    if *a == ProcessMemory::default() && *b == ProcessMemory::default() {
        return gap(1., 1.);
    }
    let mut rows = vec![
        gap(1., 10.),
        table_header(vec![
            Expanded::new(caption("Bytes")).into(),
            cell(100., caption("Baseline A")),
            cell(100., caption("Snapshot B")),
            cell(90., caption("Change")),
        ]),
        byte_row("Private".into(), a.private_bytes, b.private_bytes),
        byte_row("Resident".into(), a.resident_bytes, b.resident_bytes),
    ];
    for (label, before, after) in [
        ("Rust heap", a.heap_live_bytes, b.heap_live_bytes),
        ("GPU in use", a.gpu_allocated_bytes, b.gpu_allocated_bytes),
        ("GPU reserved", a.gpu_reserved_bytes, b.gpu_reserved_bytes),
    ] {
        if let (Some(before), Some(after)) = (before, after) {
            rows.push(byte_row(label.into(), before, after));
        }
    }
    for change in bucket_changes(first, second).into_iter().take(12) {
        rows.push(byte_row(
            format!("{} · {}", change.group, change.name),
            change.before,
            change.after,
        ));
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

/// Process totals and the per-owner tables of `snapshot`, as the Memory view
/// lays them out.
pub fn memory_breakdown_view(snapshot: &MemorySnapshot) -> Widget {
    column(memory_breakdown(snapshot))
}

/// The byte rows of the A/B comparison: process totals and the owners that
/// changed most.
pub fn memory_byte_comparison_view(first: &MemorySnapshot, second: &MemorySnapshot) -> Widget {
    byte_comparison(first, second)
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
        let breakdown = memory_breakdown(&memory);
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
        ]);
        body.extend(breakdown);
        body.extend([
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
        (Some(a), Some(b)) => column([comparison(&a.counts, &b.counts), byte_comparison(a, b)]),
        _ => ui_text(
            "Capture baseline A and comparison B to compare resource counts.",
            12.,
            text_muted(),
        ),
    };
    body.push(section(
        "Snapshot comparison",
        "Byte and inventory deltas between A and B: what grew, by owner. Growth that returns after the \n         work that caused it ends is a cache, not a leak.",
        diff,
    ));
    column(body)
}
