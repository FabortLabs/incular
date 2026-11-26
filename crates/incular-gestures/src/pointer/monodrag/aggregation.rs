use super::{Instant, Offset, PanAction, Stream};

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

#[derive(Default)]
pub(super) struct BoundaryFrame {
    stamp: Option<Instant>,
    samples: Vec<(Stream, Offset)>,
    pan_emitted: Offset,
    pending_crossing: Option<(Stream, Offset)>,
}

impl BoundaryFrame {
    pub(super) fn begin(&mut self, stamp: Instant) {
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

    pub(super) fn threshold_crossing(&mut self, stream: Stream, delta: Offset) {
        self.pending_crossing = Some((stream, delta));
    }

    pub(super) fn accepted(&mut self, stream: Stream) {
        if self
            .pending_crossing
            .is_some_and(|(candidate, _)| candidate == stream)
            && let Some((_, delta)) = self.pending_crossing.take()
        {
            self.record(stream, delta);
        }
    }

    pub(super) fn remove(&mut self, stream: Stream) {
        self.samples.retain(|(candidate, _)| *candidate != stream);
    }

    pub(super) fn update(
        &mut self,
        axis: PanAction,
        stream: Stream,
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

    fn axis_delta(&self, stream: Stream, step: f32, component: impl Fn(Offset) -> f32) -> f32 {
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

    fn record(&mut self, stream: Stream, delta: Offset) {
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
