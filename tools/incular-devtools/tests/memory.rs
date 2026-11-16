use incular::prelude::*;
use incular::runtime::Runtime;
use incular_devtools_protocol::{
    MemoryBucket, MemorySnapshot, ProcessMemory, ResourceCounts, memory_groups,
};
use incular_devtools_ui::{
    inspector::InspectorModel,
    memory::{
        bucket_changes, format_byte_delta, format_bytes, memory_groups as grouped,
        unattributed_bytes,
    },
    views::{memory_breakdown_view, memory_byte_comparison_view},
};

const MIB: u64 = 1024 * 1024;

fn bucket(group: &str, name: &str, bytes: u64, count: u64) -> MemoryBucket {
    MemoryBucket {
        group: group.into(),
        name: name.into(),
        bytes,
        count,
    }
}

fn snapshot(label: &str, private: u64, buckets: Vec<MemoryBucket>) -> MemorySnapshot {
    MemorySnapshot {
        label: label.into(),
        counts: ResourceCounts::default(),
        process: ProcessMemory {
            private_bytes: private,
            resident_bytes: private,
            peak_resident_bytes: private,
            heap_live_bytes: Some(40 * MIB),
            heap_peak_bytes: Some(60 * MIB),
            heap_live_allocations: Some(1200),
            gpu_allocated_bytes: Some(80 * MIB),
            gpu_reserved_bytes: Some(100 * MIB),
            gpu_blocks: 25,
        },
        buckets,
    }
}

#[test]
fn bytes_use_the_largest_readable_unit() {
    assert_eq!(format_bytes(0), "0 B");
    assert_eq!(format_bytes(1023), "1023 B");
    assert_eq!(format_bytes(1536), "1.5 KiB");
    assert_eq!(format_bytes(12 * MIB + MIB / 2), "12.5 MiB");
    assert_eq!(format_bytes(3 * 1024 * MIB), "3.00 GiB");
    assert_eq!(format_byte_delta(2 * i128::from(MIB)), "+2.0 MiB");
    assert_eq!(format_byte_delta(-512), "-512 B");
}

#[test]
fn groups_keep_reported_order_and_sort_their_buckets_largest_first() {
    let snapshot = snapshot(
        "manual",
        400 * MIB,
        vec![
            bucket(memory_groups::GPU_ALLOCATIONS, "glyph atlas", MIB, 1),
            bucket(memory_groups::RENDERER, "Retained layer groups", 9 * MIB, 3),
            bucket(memory_groups::GPU_ALLOCATIONS, "image texture", 30 * MIB, 4),
            bucket(
                memory_groups::CPU,
                "Decoded image pixels (CPU)",
                56 * MIB,
                44,
            ),
        ],
    );
    let groups = grouped(&snapshot);
    let names: Vec<_> = groups.iter().map(|group| group.name.as_str()).collect();
    assert_eq!(
        names,
        [
            memory_groups::GPU_ALLOCATIONS,
            memory_groups::RENDERER,
            memory_groups::CPU
        ]
    );
    assert_eq!(groups[0].total, 31 * MIB);
    assert_eq!(groups[0].buckets[0].name, "image texture");
    assert_eq!(groups[0].buckets[1].name, "glyph atlas");
}

#[test]
fn unattributed_memory_is_what_heap_and_gpu_do_not_explain() {
    let process = snapshot("manual", 400 * MIB, Vec::new()).process;
    assert_eq!(unattributed_bytes(&process), Some(260 * MIB));

    // Nothing to subtract: calling the whole process "other" would mislead.
    let unmeasured = ProcessMemory {
        private_bytes: 400 * MIB,
        ..ProcessMemory::default()
    };
    assert_eq!(unattributed_bytes(&unmeasured), None);
    assert_eq!(unattributed_bytes(&ProcessMemory::default()), None);

    // Reserved GPU memory above private bytes (a discrete GPU) never wraps.
    let discrete = ProcessMemory {
        private_bytes: 50 * MIB,
        gpu_reserved_bytes: Some(200 * MIB),
        ..ProcessMemory::default()
    };
    assert_eq!(unattributed_bytes(&discrete), Some(0));
}

#[test]
fn changes_rank_owners_by_how_much_they_moved() {
    let before = snapshot(
        "A",
        400 * MIB,
        vec![
            bucket(memory_groups::RENDERER, "Recycled target pool", 10 * MIB, 2),
            bucket(memory_groups::RENDERER, "Image textures", 30 * MIB, 4),
            bucket(memory_groups::RENDERER, "Glyph atlas pages", MIB, 1),
            bucket(memory_groups::LAYERS, "Layer group 7 800x600", 2 * MIB, 1),
        ],
    );
    let after = snapshot(
        "B",
        470 * MIB,
        vec![
            bucket(memory_groups::RENDERER, "Image textures", 90 * MIB, 12),
            bucket(memory_groups::RENDERER, "Glyph atlas pages", MIB, 1),
            bucket(
                memory_groups::CPU,
                "Decoded image pixels (CPU)",
                20 * MIB,
                8,
            ),
            bucket(memory_groups::LAYERS, "Layer group 9 800x600", 2 * MIB, 1),
        ],
    );
    let changes = bucket_changes(&before, &after);
    let summary: Vec<_> = changes
        .iter()
        .map(|change| (change.name.as_str(), change.delta()))
        .collect();
    assert_eq!(
        summary,
        [
            ("Image textures", 60 * i128::from(MIB)),
            ("Decoded image pixels (CPU)", 20 * i128::from(MIB)),
            ("Recycled target pool", -10 * i128::from(MIB)),
        ],
        "unchanged owners and individual layer textures are left out"
    );
}

#[test]
fn text_diff_names_the_bytes_and_the_owners_that_grew() {
    let mut state = InspectorModel::default();
    state.set_memory_snapshots(
        snapshot(
            "A",
            400 * MIB,
            vec![bucket(
                memory_groups::RENDERER,
                "Image textures",
                30 * MIB,
                4,
            )],
        ),
        snapshot(
            "B",
            470 * MIB,
            vec![bucket(
                memory_groups::RENDERER,
                "Image textures",
                90 * MIB,
                12,
            )],
        ),
    );
    let lines = state.memory_diff_lines();
    assert!(
        lines
            .iter()
            .any(|line| line.contains("private 400.0 MiB → 470.0 MiB (+70.0 MiB)")),
        "{lines:?}"
    );
    assert!(
        lines.iter().any(|line| line
            .contains("Renderer caches · Image textures 30.0 MiB → 90.0 MiB (+60.0 MiB)")),
        "{lines:?}"
    );
}

#[test]
fn breakdown_and_comparison_lay_out_with_and_without_reported_memory() {
    let full = snapshot(
        "A",
        400 * MIB,
        (0..20)
            .map(|index| {
                bucket(
                    memory_groups::GPU_ALLOCATIONS,
                    &format!("incular retained texture {index}"),
                    (index + 1) * MIB,
                    index + 1,
                )
            })
            .chain([
                bucket(memory_groups::RENDERER, "Retained layer groups", 9 * MIB, 3),
                bucket(
                    memory_groups::CPU,
                    "Decoded image pixels (CPU)",
                    56 * MIB,
                    44,
                ),
            ])
            .collect(),
    );
    // What a target built before memory breakdowns existed sends.
    let legacy = MemorySnapshot {
        label: "B".into(),
        counts: ResourceCounts::default(),
        process: ProcessMemory::default(),
        buckets: Vec::new(),
    };
    for (first, second) in [(&full, &full), (&full, &legacy), (&legacy, &legacy)] {
        let root: Widget = Directionality::new(
            TextDirection::Ltr,
            SingleChildScrollView::new(Column::new([
                memory_breakdown_view(first),
                memory_byte_comparison_view(first, second),
            ])),
        )
        .into();
        let mut runtime = Runtime::new(root).unwrap();
        runtime
            .run_frame(Constraints::tight(Size::new(1100., 800.)))
            .unwrap();
    }
}
