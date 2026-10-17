use incular_devtools_protocol::{
    DeepFrameTrace, DevWidgetId, FrameRecordEvent, FrameTimingsWire, TracePhase,
};
use std::collections::{HashMap, HashSet};

/// CPU statistics for an explicit set of observed frames, using nearest-rank p95.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FrameStatistics {
    pub samples: usize,
    pub average_us: f64,
    pub p95_us: u32,
    pub worst_us: u32,
    pub budgeted_samples: usize,
    pub over_budget: usize,
}

impl FrameStatistics {
    pub fn from_frames<'a>(frames: impl IntoIterator<Item = &'a FrameRecordEvent>) -> Self {
        let mut times = Vec::new();
        let mut budgeted_samples = 0;
        let mut over_budget = 0;
        for frame in frames {
            times.push(frame.timings.cpu_total);
            if frame.budget_us.is_some_and(|budget| budget > 0) {
                budgeted_samples += 1;
                over_budget += usize::from(frame.over_budget);
            }
        }
        if times.is_empty() {
            return Self::default();
        }
        times.sort_unstable();
        Self {
            samples: times.len(),
            average_us: times.iter().map(|value| f64::from(*value)).sum::<f64>()
                / times.len() as f64,
            p95_us: times[(times.len() * 95).div_ceil(100) - 1],
            worst_us: *times.last().expect("nonempty timings"),
            budgeted_samples,
            over_budget,
        }
    }

    pub fn jank_percent(self) -> Option<f32> {
        (self.budgeted_samples > 0)
            .then(|| self.over_budget as f32 * 100. / self.budgeted_samples as f32)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct FlameBox {
    pub event: usize,
    pub node: DevWidgetId,
    pub phase: TracePhase,
    pub x: f32,
    pub width: f32,
    pub depth: u16,
    pub duration_us: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RankedTrace {
    pub node: DevWidgetId,
    pub phase: TracePhase,
    pub total_us: u64,
    pub count: u32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TraceRange {
    CurrentFrame,
    SelectedRange,
    #[default]
    EntireRecording,
}

pub fn flamegraph_boxes(
    trace: &DeepFrameTrace,
    phase: Option<TracePhase>,
    width: f32,
) -> Vec<FlameBox> {
    let selected = trace
        .events
        .iter()
        .enumerate()
        .filter(|(_, event)| phase.is_none_or(|phase| event.phase == phase))
        .collect::<Vec<_>>();
    let Some(start) = selected.iter().map(|(_, event)| event.start_us).min() else {
        return Vec::new();
    };
    let end = selected
        .iter()
        .map(|(_, event)| event.start_us.saturating_add(event.duration_us))
        .max()
        .unwrap_or(start);
    let extent = end.saturating_sub(start).max(1) as f32;
    selected
        .into_iter()
        .map(|(index, event)| {
            let mut depth = 0_u16;
            let mut parent = event.parent;
            let mut visited = HashSet::new();
            while let Some(parent_index) = parent {
                if !visited.insert(parent_index) {
                    break;
                }
                let Some(parent_event) = trace.events.get(parent_index as usize) else {
                    break;
                };
                if phase.is_none_or(|phase| parent_event.phase == phase) {
                    depth = depth.saturating_add(1);
                }
                parent = parent_event.parent;
            }
            FlameBox {
                event: index,
                node: event.node,
                phase: event.phase,
                x: (event.start_us.saturating_sub(start) as f32 / extent) * width,
                width: (event.duration_us as f32 / extent * width).max(1.),
                depth,
                duration_us: event.duration_us,
            }
        })
        .collect()
}

pub fn rank_traces<'a>(
    traces: impl IntoIterator<Item = &'a DeepFrameTrace>,
    phase: Option<TracePhase>,
) -> Vec<RankedTrace> {
    let mut totals = HashMap::<(DevWidgetId, TracePhase), (u64, u32)>::new();
    for event in traces.into_iter().flat_map(|trace| trace.events.iter()) {
        if phase.is_some_and(|phase| phase != event.phase) {
            continue;
        }
        let total = totals.entry((event.node, event.phase)).or_default();
        total.0 = total.0.saturating_add(u64::from(event.duration_us));
        total.1 = total.1.saturating_add(1);
    }
    let mut ranked = totals
        .into_iter()
        .map(|((node, phase), (total_us, count))| RankedTrace {
            node,
            phase,
            total_us,
            count,
        })
        .collect::<Vec<_>>();
    ranked.sort_by(|left, right| {
        right
            .total_us
            .cmp(&left.total_us)
            .then_with(|| right.count.cmp(&left.count))
            .then_with(|| left.node.index().cmp(&right.node.index()))
    });
    ranked
}

/// Stacking order of the per-frame CPU bar, bottom to top.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FramePhase {
    Build,
    Layout,
    Paint,
    Semantics,
    Composite,
    /// CPU time not attributed to a named phase (`cpu_total` minus the phases).
    Other,
}

impl FramePhase {
    pub const ALL: [Self; 6] = [
        Self::Build,
        Self::Layout,
        Self::Paint,
        Self::Semantics,
        Self::Composite,
        Self::Other,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Build => "Build",
            Self::Layout => "Layout",
            Self::Paint => "Paint",
            Self::Semantics => "Semantics",
            Self::Composite => "Composite",
            Self::Other => "Other",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PhaseSegment {
    pub phase: FramePhase,
    pub micros: u32,
}

/// Splits a frame's CPU total into the stacked segments, always returning all
/// six phases in [`FramePhase::ALL`] order. `Other` absorbs whatever the named
/// phases do not explain and saturates at zero when they overshoot the total.
pub fn phase_segments(timings: &FrameTimingsWire) -> [PhaseSegment; 6] {
    let named = [
        timings.build,
        timings.layout,
        timings.paint,
        timings.semantics,
        timings.composite,
    ];
    let sum = named
        .iter()
        .fold(0_u32, |sum, value| sum.saturating_add(*value));
    let values = [
        named[0],
        named[1],
        named[2],
        named[3],
        named[4],
        timings.cpu_total.saturating_sub(sum),
    ];
    std::array::from_fn(|index| PhaseSegment {
        phase: FramePhase::ALL[index],
        micros: values[index],
    })
}

/// Height the stacked bar should occupy: the larger of `cpu_total` and the sum
/// of its segments, so overshooting phases are never clipped.
pub fn stacked_total(timings: &FrameTimingsWire) -> u32 {
    phase_segments(timings)
        .iter()
        .fold(0_u32, |sum, segment| sum.saturating_add(segment.micros))
}

/// A frame is janky only when the target reported a positive budget and
/// flagged the frame as over it (matches [`FrameStatistics`]).
pub fn is_janky(frame: &FrameRecordEvent) -> bool {
    frame.budget_us.is_some_and(|budget| budget > 0) && frame.over_budget
}

/// Percentage of the frame's CPU total taken by `micros`; `None` for empty frames.
pub fn percent_of_frame(micros: u32, timings: &FrameTimingsWire) -> Option<f32> {
    (timings.cpu_total > 0).then(|| micros as f32 * 100. / timings.cpu_total as f32)
}

/// Formats microseconds as `µs` below one millisecond and `ms` above it.
pub fn format_micros(micros: u64) -> String {
    if micros >= 1000 {
        format!("{:.2} ms", micros as f64 / 1000.)
    } else {
        format!("{micros} µs")
    }
}
