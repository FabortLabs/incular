//! The allocator summary a memory snapshot reports.

use incular_wgpu::{GpuAllocatorSummary, GpuMemoryEntry};

fn allocation(name: &str, size: u64) -> wgpu::wgt::AllocationReport {
    wgpu::wgt::AllocationReport {
        name: name.into(),
        offset: 0,
        size,
    }
}

#[test]
fn allocations_group_by_label_largest_first() {
    let report = wgpu::AllocatorReport {
        allocations: vec![
            allocation("incular retained opacity group", 4 << 20),
            allocation("incular shared image texture", 8 << 20),
            allocation("incular retained opacity group", 3 << 20),
            allocation("", 1 << 20),
            allocation("incular shared glyph atlas page", 1 << 20),
        ],
        blocks: vec![
            wgpu::wgt::MemoryBlockReport {
                size: 16 << 20,
                allocations: 0..3,
            },
            wgpu::wgt::MemoryBlockReport {
                size: 4 << 20,
                allocations: 3..5,
            },
        ],
        total_allocated_bytes: 17 << 20,
        total_reserved_bytes: 20 << 20,
    };
    let summary = GpuAllocatorSummary::from_report(&report);
    assert_eq!(summary.allocated_bytes, 17 << 20);
    assert_eq!(summary.reserved_bytes, 20 << 20);
    assert_eq!(summary.block_count, 2);
    let entry = |name: &str, bytes: u64, count: u64| GpuMemoryEntry {
        name: name.into(),
        bytes,
        count,
    };
    assert_eq!(
        summary.allocations,
        [
            entry("incular shared image texture", 8 << 20, 1),
            entry("incular retained opacity group", 7 << 20, 2),
            // Equal sizes order by name, so a report is stable.
            entry("(unlabeled)", 1 << 20, 1),
            entry("incular shared glyph atlas page", 1 << 20, 1),
        ]
    );
}

#[test]
fn an_empty_report_summarizes_to_nothing() {
    let summary = GpuAllocatorSummary::from_report(&wgpu::AllocatorReport {
        allocations: Vec::new(),
        blocks: Vec::new(),
        total_allocated_bytes: 0,
        total_reserved_bytes: 0,
    });
    assert_eq!(summary, GpuAllocatorSummary::default());
}
