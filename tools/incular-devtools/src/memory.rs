//! Memory snapshots as the Memory view presents them: byte formatting, the
//! buckets of a snapshot by group, what the process holds that nothing
//! reported, and what changed between two snapshots.

use incular_devtools_protocol::{MemoryBucket, MemorySnapshot, ProcessMemory};

/// `bytes` in the largest binary unit that keeps it readable.
#[must_use]
pub fn format_bytes(bytes: u64) -> String {
    const KIB: f64 = 1024.;
    let value = bytes as f64;
    if value >= KIB * KIB * KIB {
        format!("{:.2} GiB", value / (KIB * KIB * KIB))
    } else if value >= KIB * KIB {
        format!("{:.1} MiB", value / (KIB * KIB))
    } else if value >= KIB {
        format!("{:.1} KiB", value / KIB)
    } else {
        format!("{bytes} B")
    }
}

/// A signed byte change, with its sign always shown.
#[must_use]
pub fn format_byte_delta(delta: i128) -> String {
    let sign = if delta < 0 { '-' } else { '+' };
    let magnitude = u64::try_from(delta.unsigned_abs()).unwrap_or(u64::MAX);
    format!("{sign}{}", format_bytes(magnitude))
}

/// The buckets of one group of a snapshot, largest first.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemoryGroup {
    pub name: String,
    /// Sum of the group's buckets.
    pub total: u64,
    pub buckets: Vec<MemoryBucket>,
}

/// The groups of `snapshot`, in the order the target reported them.
#[must_use]
pub fn memory_groups(snapshot: &MemorySnapshot) -> Vec<MemoryGroup> {
    let mut groups: Vec<MemoryGroup> = Vec::new();
    for bucket in &snapshot.buckets {
        let index = groups
            .iter()
            .position(|group| group.name == bucket.group)
            .unwrap_or_else(|| {
                groups.push(MemoryGroup {
                    name: bucket.group.clone(),
                    total: 0,
                    buckets: Vec::new(),
                });
                groups.len() - 1
            });
        let group = &mut groups[index];
        group.total = group.total.saturating_add(bucket.bytes);
        group.buckets.push(bucket.clone());
    }
    for group in &mut groups {
        group
            .buckets
            .sort_by(|a, b| b.bytes.cmp(&a.bytes).then_with(|| a.name.cmp(&b.name)));
    }
    groups
}

/// Private bytes neither the Rust heap nor the GPU allocator accounts for:
/// native libraries, the graphics driver, thread stacks and the binary's own
/// data. `None` when the target reported no private bytes, or when neither
/// the heap nor the GPU allocator was measured, which would attribute the
/// whole process to "other".
#[must_use]
pub fn unattributed_bytes(process: &ProcessMemory) -> Option<u64> {
    if process.private_bytes == 0
        || (process.heap_live_bytes.is_none() && process.gpu_reserved_bytes.is_none())
    {
        return None;
    }
    Some(
        process
            .private_bytes
            .saturating_sub(process.heap_live_bytes.unwrap_or(0))
            .saturating_sub(process.gpu_reserved_bytes.unwrap_or(0)),
    )
}

/// One bucket's bytes in two snapshots.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BucketChange {
    pub group: String,
    pub name: String,
    pub before: u64,
    pub after: u64,
}

impl BucketChange {
    #[must_use]
    pub fn delta(&self) -> i128 {
        i128::from(self.after) - i128::from(self.before)
    }
}

/// Buckets whose bytes differ between `before` and `after`, largest change
/// first. A bucket present in only one snapshot counts as zero in the other.
/// The per-layer group is left out: its buckets name individual textures
/// that come and go, and the renderer caches group already totals them.
#[must_use]
pub fn bucket_changes(before: &MemorySnapshot, after: &MemorySnapshot) -> Vec<BucketChange> {
    let tracked =
        |bucket: &&MemoryBucket| bucket.group != incular_devtools_protocol::memory_groups::LAYERS;
    let mut changes: Vec<BucketChange> = Vec::new();
    for bucket in before.buckets.iter().filter(tracked) {
        merge(&mut changes, bucket).before += bucket.bytes;
    }
    for bucket in after.buckets.iter().filter(tracked) {
        merge(&mut changes, bucket).after += bucket.bytes;
    }
    changes.retain(|change| change.before != change.after);
    changes.sort_by(|a, b| {
        b.delta()
            .unsigned_abs()
            .cmp(&a.delta().unsigned_abs())
            .then_with(|| a.name.cmp(&b.name))
    });
    changes
}

fn merge<'a>(changes: &'a mut Vec<BucketChange>, bucket: &MemoryBucket) -> &'a mut BucketChange {
    let index = changes
        .iter()
        .position(|change| change.group == bucket.group && change.name == bucket.name)
        .unwrap_or_else(|| {
            changes.push(BucketChange {
                group: bucket.group.clone(),
                name: bucket.name.clone(),
                before: 0,
                after: 0,
            });
            changes.len() - 1
        });
    &mut changes[index]
}
