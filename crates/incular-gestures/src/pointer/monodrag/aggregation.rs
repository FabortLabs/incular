use super::{Instant, Offset, PanAction};

/// How a drag combines movement from several accepted contacts.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum MultitouchDragStrategy {
    /// Only the most recently accepted, still-active pointer moves the drag.
    #[default]
    LatestPointer,
    /// Axis drags follow the directional boundaries in each system frame.
    /// A free pan averages the movement across accepted contacts.
    AverageBoundaryPointers,
    /// Every accepted pointer contributes its own displacement.
    SumAllPointers,
}

/// Frame accumulator shared by recognizers and retained scroll groups.
#[doc(hidden)]
pub struct DragBoundaryFrame<K> {
    stamp: Option<Instant>,
    samples: Vec<(K, Offset)>,
    pan_emitted: Offset,
    pending_crossing: Option<(K, Offset)>,
}

impl<K> Default for DragBoundaryFrame<K> {
    fn default() -> Self {
        Self {
            stamp: None,
            samples: Vec::new(),
            pan_emitted: Offset::ZERO,
            pending_crossing: None,
        }
    }
}
impl<K: Copy + Eq> DragBoundaryFrame<K> {
    pub fn begin(&mut self, stamp: Instant) {
        if self.stamp != Some(stamp) {
            self.stamp = Some(stamp);
            self.samples.clear();
            self.pan_emitted = Offset::ZERO;
            self.pending_crossing = None;
        }
    }

    pub(super) fn clear_crossing(&mut self) {
        self.pending_crossing = None;
    }

    pub(super) fn threshold_crossing(&mut self, stream: K, delta: Offset) {
        self.pending_crossing = Some((stream, delta));
    }

    pub(super) fn accepted(&mut self, stream: K) {
        if self
            .pending_crossing
            .is_some_and(|(candidate, _)| candidate == stream)
            && let Some((_, delta)) = self.pending_crossing.take()
        {
            self.record(stream, delta);
        }
    }

    pub fn remove(&mut self, stream: K) {
        self.samples.retain(|(candidate, _)| *candidate != stream);
    }

    pub(super) fn update(
        &mut self,
        axis: PanAction,
        stream: K,
        local_delta: Offset,
        accepted: usize,
    ) -> Offset {
        if local_delta == Offset::ZERO {
            return Offset::ZERO;
        }
        let resolved = match axis {
            PanAction::Horizontal => {
                Offset::new(self.axis_delta(stream, local_delta.x, |d| d.x), 0.)
            }
            PanAction::Vertical => Offset::new(0., self.axis_delta(stream, local_delta.y, |d| d.y)),
            PanAction::Pan => {
                let total = self
                    .samples
                    .iter()
                    .fold(local_delta, |sum, (_, delta)| sum + *delta);
                let count = accepted as f32;
                let average = Offset::new(total.x / count, total.y / count);
                let output = average - self.pan_emitted;
                self.pan_emitted = average;
                output
            }
        };
        self.record(stream, local_delta);
        resolved
    }

    /// Computes and records an update for a group on one fixed scroll axis.
    pub fn update_scroll_axis(&mut self, stream: K, delta: f32) -> f32 {
        self.update(PanAction::Horizontal, stream, Offset::new(delta, 0.), 1)
            .x
    }

    fn axis_delta(&self, stream: K, step: f32, component: impl Fn(Offset) -> f32) -> f32 {
        if self.samples.is_empty() || step == 0. {
            return step;
        }
        // Compare cumulative movement in the direction of this update. A
        // contact already at that boundary contributes the whole step.
        let sign = step.signum();
        let mut maximum = -1.;
        let mut boundary = stream;
        let mut current = 0.;
        for &(pointer, delta) in &self.samples {
            let magnitude = (component(delta) * sign).max(0.);
            if magnitude > maximum {
                maximum = magnitude;
                boundary = pointer;
            }
            if pointer == stream {
                current = magnitude;
            }
        }
        if boundary == stream {
            step
        } else {
            sign * (current + step.abs() - maximum).max(0.)
        }
    }

    fn record(&mut self, stream: K, delta: Offset) {
        if delta == Offset::ZERO {
            return;
        }
        if let Some((_, sum)) = self
            .samples
            .iter_mut()
            .find(|(candidate, _)| *candidate == stream)
        {
            *sum = *sum + delta;
        } else {
            self.samples.push((stream, delta));
        }
    }
}
