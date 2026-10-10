use incular_devtools_protocol::{DeepFrameTrace, DevWidgetId, FrameRecordEvent, TracePhase};
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
