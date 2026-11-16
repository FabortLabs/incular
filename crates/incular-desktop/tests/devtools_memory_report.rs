#![cfg(feature = "devtools")]

//! The memory snapshot a DevTools target reports and the text report
//! `INCULAR_MEMORY_LOG` writes from it.

use incular_core::{Color, Size};
use incular_desktop::devtools_runner::{RendererMemory, format_memory_report, memory_snapshot};
use incular_devtools_protocol::memory_groups;
use incular_runtime::Application;
use incular_wgpu::{GpuAllocatorSummary, GpuMemoryEntry};
use incular_widgets::Widget;

const MIB: u64 = 1024 * 1024;

fn entry(name: &str, bytes: u64, count: u64) -> GpuMemoryEntry {
    GpuMemoryEntry {
        name: name.into(),
        bytes,
        count,
    }
}

fn renderer_memory() -> RendererMemory {
    RendererMemory {
        allocator: Some(GpuAllocatorSummary {
            allocated_bytes: 60 * MIB,
            reserved_bytes: 80 * MIB,
            block_count: 20,
            allocations: vec![
                entry("incular shared image texture", 40 * MIB, 5),
                entry("incular retained opacity group", 20 * MIB, 2),
            ],
        }),
        shared: vec![
            entry("Image textures", 40 * MIB, 5),
            entry("Gradient lookups", MIB / 4, 3),
            entry("Glyph atlas pages", MIB, 1),
        ],
        retained: vec![
            entry("Retained effect results", 2 * MIB, 4),
            entry("Retained layer groups", 20 * MIB, 2),
            entry("Path meshes", MIB / 2, 12),
            entry("Path meshes (CPU)", MIB / 8, 12),
        ],
        layers: vec![entry(
            "Layer group LayerId(7) 1536x768, idle 0 frames",
            5 * MIB,
            1,
        )],
        decoded_images: entry("Decoded image pixels (CPU)", 56 * MIB, 44),
    }
}

fn application() -> Application {
    Application::new(|_| Widget::box_(Size::new(40., 40.), Color::WHITE)).unwrap()
}

#[test]
fn snapshot_reports_process_totals_inventories_and_owners() {
    let snapshot = memory_snapshot("manual".into(), &application(), &renderer_memory());
    assert_eq!(snapshot.label, "manual");

    // The allocator's totals travel with the process figures.
    assert_eq!(snapshot.process.gpu_allocated_bytes, Some(60 * MIB));
    assert_eq!(snapshot.process.gpu_reserved_bytes, Some(80 * MIB));
    assert_eq!(snapshot.process.gpu_blocks, 20);
    if cfg!(any(windows, target_os = "linux")) {
        assert!(snapshot.process.resident_bytes > 0);
        assert_eq!(
            snapshot.counts.rss_mb as u64,
            snapshot.process.resident_bytes / MIB
        );
    }

    // Renderer inventories come from the caches that own them.
    assert_eq!(snapshot.counts.image_resources, 5);
    assert_eq!(snapshot.counts.gradient_resources, 3);
    assert_eq!(snapshot.counts.glyph_atlas_pages, 1);
    assert_eq!(snapshot.counts.path_meshes, 12);
    assert_eq!(snapshot.counts.offscreen_bytes as u64, 20 * MIB);
    assert_eq!(snapshot.counts.effect_cached_bytes as u64, 2 * MIB);

    let group = |name: &str| -> Vec<(&str, u64)> {
        snapshot
            .buckets
            .iter()
            .filter(|bucket| bucket.group == name)
            .map(|bucket| (bucket.name.as_str(), bucket.bytes))
            .collect()
    };
    assert_eq!(
        group(memory_groups::GPU_ALLOCATIONS),
        [
            ("incular shared image texture", 40 * MIB),
            ("incular retained opacity group", 20 * MIB),
        ]
    );
    // Shared and per-window caches in one list, largest first, without
    // the CPU-side owners.
    assert_eq!(
        group(memory_groups::RENDERER),
        [
            ("Image textures", 40 * MIB),
            ("Retained layer groups", 20 * MIB),
            ("Retained effect results", 2 * MIB),
            ("Glyph atlas pages", MIB),
            ("Path meshes", MIB / 2),
            ("Gradient lookups", MIB / 4),
        ]
    );
    assert_eq!(group(memory_groups::LAYERS).len(), 1);
    assert_eq!(
        group(memory_groups::CPU),
        [
            ("Decoded image pixels (CPU)", 56 * MIB),
            ("Path meshes (CPU)", MIB / 8),
        ]
    );
}

#[test]
fn snapshot_without_a_gpu_report_claims_nothing_about_the_gpu() {
    let snapshot = memory_snapshot("manual".into(), &application(), &RendererMemory::default());
    assert_eq!(snapshot.process.gpu_allocated_bytes, None);
    assert_eq!(snapshot.process.gpu_reserved_bytes, None);
    assert!(snapshot.buckets.is_empty());
}

#[test]
fn text_report_totals_each_group_above_its_owners() {
    let snapshot = memory_snapshot("manual".into(), &application(), &renderer_memory());
    let report = format_memory_report(&snapshot);
    let lines: Vec<&str> = report.lines().collect();
    assert!(
        lines
            .iter()
            .all(|line| line.starts_with("[incular memory]")),
        "every line carries the prefix a log can be filtered by:\n{report}"
    );
    assert!(
        lines[0].contains("GPU allocated 60.0 MiB in 80.0 MiB reserved (20 blocks)"),
        "{}",
        lines[0]
    );
    let position = |needle: &str| {
        lines
            .iter()
            .position(|line| line.contains(needle))
            .unwrap_or_else(|| panic!("missing {needle:?} in:\n{report}"))
    };
    let allocations = position("GPU allocations: 60.0 MiB");
    let caches = position("Renderer caches: 63.8 MiB");
    let cpu = position("CPU: 56.1 MiB");
    assert!(allocations < caches && caches < cpu);
    let image = position("x5     incular shared image texture");
    assert!(allocations < image && image < caches);
    assert!(lines[image].contains("40.00 MiB"), "{}", lines[image]);
}
