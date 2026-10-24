use incular_core::time::Instant;
use std::collections::VecDeque;

use incular_core::Offset;

/// Flutter's `VelocityEstimate`: the fitted velocity and the displacement
/// across the samples that produced it. A default estimate has no motion.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct VelocityEstimate {
    pub(crate) pixels_per_second: Offset,
    pub(crate) offset: Offset,
}

/// Which Flutter velocity tracker estimates a drag's release velocity
/// (`DragGestureRecognizer.velocityTrackerBuilder`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum VelocityTrackerKind {
    /// `VelocityTracker`: a degree-2 least-squares fit over the last 100 ms.
    /// Flutter's default, and its scroll behavior's on Android, Fuchsia,
    /// Linux and Windows.
    #[default]
    LeastSquares,
    /// `IOSScrollViewFlingVelocityTracker`: a weighted average of the last
    /// three sample-to-sample velocities, as an iOS scroll view flings.
    IosScrollViewFling,
    /// `MacOSScrollViewFlingVelocityTracker`: the iOS tracker with macOS
    /// weights.
    MacOsScrollViewFling,
}

/// Port of Flutter's `VelocityTracker` estimate: a degree-2 least-squares fit
/// over the recent position samples of one pointer, or one of its scroll view
/// fling trackers.
#[derive(Clone, Debug, Default)]
pub(crate) struct VelocityTracker {
    samples: VecDeque<(Instant, Offset)>,
    kind: VelocityTrackerKind,
}

impl VelocityTracker {
    const HISTORY_SIZE: usize = 20;
    const HORIZON_MILLISECONDS: f64 = 100.0;
    const ASSUME_POINTER_MOVE_STOPPED_MILLISECONDS: f64 = 40.0;
    const MIN_SAMPLE_SIZE: usize = 3;

    pub(crate) fn new(kind: VelocityTrackerKind) -> Self {
        Self {
            samples: VecDeque::new(),
            kind,
        }
    }

    pub(crate) const fn kind(&self) -> VelocityTrackerKind {
        self.kind
    }

    pub(crate) fn add_position(&mut self, time: Instant, position: Offset) {
        if self.samples.len() == Self::HISTORY_SIZE {
            self.samples.pop_front();
        }
        self.samples.push_back((time, position));
    }

    /// Returns the estimate at `now`. The velocity is zero when there is no
    /// usable fit (no samples, a pointer that stopped, or too few samples).
    pub(crate) fn estimate_at(&self, now: Instant) -> VelocityEstimate {
        let Some(&(newest_time, newest_point)) = self.samples.back() else {
            return VelocityEstimate::default();
        };
        // Flutter reports no motion when the pointer has been still for more than 40 ms.
        if now.saturating_duration_since(newest_time).as_millis() as f64
            > Self::ASSUME_POINTER_MOVE_STOPPED_MILLISECONDS
        {
            return VelocityEstimate::default();
        }
        let weights = match self.kind {
            VelocityTrackerKind::LeastSquares => None,
            VelocityTrackerKind::IosScrollViewFling => Some([0.6, 0.35, 0.05]),
            VelocityTrackerKind::MacOsScrollViewFling => Some([0.15, 0.65, 0.2]),
        };
        if let Some([earliest, previous, latest]) = weights {
            let oldest_point = self.samples.front().map_or(newest_point, |sample| sample.1);
            let [a, b, c] = [
                self.previous_velocity_at(2),
                self.previous_velocity_at(1),
                self.previous_velocity_at(0),
            ];
            return VelocityEstimate {
                pixels_per_second: Offset::new(
                    a.x * earliest + b.x * previous + c.x * latest,
                    a.y * earliest + b.y * previous + c.y * latest,
                ),
                offset: Offset::new(
                    newest_point.x - oldest_point.x,
                    newest_point.y - oldest_point.y,
                ),
            };
        }

        let mut x = Vec::new();
        let mut y = Vec::new();
        let mut time = Vec::new();
        let mut oldest_point = newest_point;
        let mut previous_time = newest_time;
        for &(sample_time, point) in self.samples.iter().rev() {
            let age = micros_between(sample_time, newest_time) / 1000.0;
            let delta = micros_between(previous_time, sample_time).abs() / 1000.0;
            previous_time = sample_time;
            if age > Self::HORIZON_MILLISECONDS
                || delta > Self::ASSUME_POINTER_MOVE_STOPPED_MILLISECONDS
            {
                break;
            }
            oldest_point = point;
            x.push(f64::from(point.x));
            y.push(f64::from(point.y));
            time.push(-age);
        }
        let offset = Offset::new(
            newest_point.x - oldest_point.x,
            newest_point.y - oldest_point.y,
        );

        if time.len() >= Self::MIN_SAMPLE_SIZE
            && let (Some(slope_x), Some(slope_y)) =
                (solve_linear_slope(&time, &x), solve_linear_slope(&time, &y))
        {
            // The slopes are in pixels per millisecond; report pixels per second.
            return VelocityEstimate {
                pixels_per_second: Offset::new(
                    (slope_x * 1000.0) as f32,
                    (slope_y * 1000.0) as f32,
                ),
                offset,
            };
        }
        VelocityEstimate {
            pixels_per_second: Offset::ZERO,
            offset,
        }
    }
}

impl VelocityTracker {
    /// `IOSScrollViewFlingVelocityTracker._previousVelocityAt(-back)`: the
    /// velocity between the samples `back` and `back + 1` before the newest,
    /// zero where the history is too short or the samples share a time.
    fn previous_velocity_at(&self, back: usize) -> Offset {
        let len = self.samples.len();
        let (Some(end), Some(start)) = (
            len.checked_sub(1 + back).map(|index| self.samples[index]),
            len.checked_sub(2 + back).map(|index| self.samples[index]),
        ) else {
            return Offset::ZERO;
        };
        let dt = micros_between(start.0, end.0);
        if dt <= 0.0 {
            return Offset::ZERO;
        }
        // Flutter converts microseconds to milliseconds first.
        let scale = (1000.0 / (dt / 1000.0)) as f32;
        Offset::new((end.1.x - start.1.x) * scale, (end.1.y - start.1.y) * scale)
    }
}

/// Signed microseconds from `from` to `to` (`to - from`), as Dart's `inMicroseconds`.
fn micros_between(from: Instant, to: Instant) -> f64 {
    if to >= from {
        to.duration_since(from).as_micros() as f64
    } else {
        -(from.duration_since(to).as_micros() as f64)
    }
}

/// Returns the linear coefficient of a degree-2 least-squares fit with unit
/// weights, following Flutter's `LeastSquaresSolver.solve(2)`.
fn solve_linear_slope(x: &[f64], y: &[f64]) -> Option<f64> {
    const PRECISION_ERROR_TOLERANCE: f64 = 1e-10;

    let m = x.len();
    // Design matrix rows are 1, x and x^2 (all weights are 1).
    let a = [
        vec![1.0; m],
        x.to_vec(),
        x.iter().map(|value| value * value).collect::<Vec<f64>>(),
    ];

    // Gram-Schmidt orthonormalisation of the design matrix: A = Q R.
    let mut q = [vec![0.0; m], vec![0.0; m], vec![0.0; m]];
    let mut r = [[0.0_f64; 3]; 3];
    for j in 0..3 {
        q[j].copy_from_slice(&a[j]);
        for i in 0..j {
            let previous = q[i].clone();
            let projection = dot(&q[j], &previous);
            for (value, basis) in q[j].iter_mut().zip(&previous) {
                *value -= projection * basis;
            }
        }
        let norm = dot(&q[j], &q[j]).sqrt();
        if norm < PRECISION_ERROR_TOLERANCE {
            return None;
        }
        let inverse_norm = 1.0 / norm;
        for value in q[j].iter_mut() {
            *value *= inverse_norm;
        }
        for (i, column) in a.iter().enumerate() {
            r[j][i] = if i < j { 0.0 } else { dot(&q[j], column) };
        }
    }

    // Solve R B = Q^T Y by back substitution; with unit weights W Y = Y.
    let mut coefficients = [0.0_f64; 3];
    for i in (0..3).rev() {
        coefficients[i] = dot(&q[i], y);
        for j in ((i + 1)..3).rev() {
            coefficients[i] -= r[i][j] * coefficients[j];
        }
        coefficients[i] /= r[i][i];
    }
    Some(coefficients[1])
}

fn dot(left: &[f64], right: &[f64]) -> f64 {
    left.iter().zip(right).fold(0.0, |sum, (a, b)| sum + a * b)
}
