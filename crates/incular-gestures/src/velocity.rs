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

    pub(crate) fn add_position(&mut self, time: Instant, position: Offset) {
        if self.samples.len() == Self::HISTORY_SIZE {
            self.samples.pop_front();
        }
        self.samples.push_back((time, position));
    }

    /// Forgets every sample, keeping the buffer and the kind.
    pub(crate) fn clear(&mut self) {
        self.samples.clear();
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

        // The ring history is bounded, so fitting a drag does not need
        // heap scratch buffers.
        let mut x = [0.0; Self::HISTORY_SIZE];
        let mut y = [0.0; Self::HISTORY_SIZE];
        let mut time = [0.0; Self::HISTORY_SIZE];
        let mut count = 0;
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
            x[count] = f64::from(point.x);
            y[count] = f64::from(point.y);
            time[count] = -age;
            count += 1;
        }
        let offset = Offset::new(
            newest_point.x - oldest_point.x,
            newest_point.y - oldest_point.y,
        );

        if count >= Self::MIN_SAMPLE_SIZE
            && let (Some(slope_x), Some(slope_y)) = (
                solve_linear_slope(&time[..count], &x[..count]),
                solve_linear_slope(&time[..count], &y[..count]),
            )
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
    // Design matrix rows are 1, x and x^2 (all weights are 1). The fit holds
    // at most `HISTORY_SIZE` samples, so the scratch rows are fixed arrays.
    let mut a = [[0.0_f64; VelocityTracker::HISTORY_SIZE]; 3];
    a[0][..m].fill(1.0);
    for (i, &value) in x.iter().enumerate() {
        a[1][i] = value;
        a[2][i] = value * value;
    }

    // Gram-Schmidt orthonormalisation of the design matrix: A = Q R.
    let mut q = [[0.0_f64; VelocityTracker::HISTORY_SIZE]; 3];
    let mut r = [[0.0_f64; 3]; 3];
    for j in 0..3 {
        let (done, rest) = q.split_at_mut(j);
        let current = &mut rest[0][..m];
        current.copy_from_slice(&a[j][..m]);
        for basis in done {
            let basis = &basis[..m];
            let projection = dot(current, basis);
            for (value, b) in current.iter_mut().zip(basis) {
                *value -= projection * b;
            }
        }
        let norm = dot(current, current).sqrt();
        if norm < PRECISION_ERROR_TOLERANCE {
            return None;
        }
        let inverse_norm = 1.0 / norm;
        for value in current.iter_mut() {
            *value *= inverse_norm;
        }
        for (i, column) in a.iter().enumerate() {
            r[j][i] = if i < j {
                0.0
            } else {
                dot(current, &column[..m])
            };
        }
    }

    // Solve R B = Q^T Y by back substitution; with unit weights W Y = Y.
    let mut coefficients = [0.0_f64; 3];
    for i in (0..3).rev() {
        coefficients[i] = dot(&q[i][..m], y);
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

#[cfg(test)]
mod flutter350_velocity {
    use super::*;
    use std::time::Duration;

    #[test]
    fn recorded_gestures_match_all_thirteen_flutter_release_velocities() {
        let trace = include_str!("../tests/fixtures/flutter350_velocity/trace.csv");
        let expected = [
            (219.59280094228163, 1304.701682306001),
            (355.71046950050845, 967.2112857054104),
            (12.657970884022308, -36.90447839251946),
            (714.1399654786744, -2561.534447931869),
            (-19.668121066218564, -2910.105747052462),
            (646.8690114934209, 2976.977762577527),
            (396.6988447819592, 2106.225572911095),
            (298.31594440044495, -3660.8315955215294),
            (-1.7334232785165882, -3288.13174127454),
            (384.6361280392334, -2645.6612524779835),
            (176.37900397918557, 2711.2542876273264),
            (396.9328560260098, 4280.651578291764),
            (-71.51939428321249, 3716.7385187526947),
        ];
        let start = Instant::now();
        let mut tracker = VelocityTracker::new(VelocityTrackerKind::LeastSquares);
        let mut last_sample = start;
        let mut releases = 0;
        let mut records = 0;
        for row in trace.lines().filter(|row| !row.starts_with('#')) {
            let [phase, ms, _pointer, x, y]: [&str; 5] = row
                .split(',')
                .collect::<Vec<_>>()
                .try_into()
                .expect("valid trace row");
            let time = start + Duration::from_millis(ms.parse().unwrap());
            assert!(time >= last_sample);
            records += 1;
            match phase {
                "down" | "move" => {
                    tracker.add_position(time, Offset::new(x.parse().unwrap(), y.parse().unwrap()));
                    last_sample = time;
                }
                "up" => {
                    // The original replays the trace immediately: its wall
                    // clock has not stopped the pointer at the release.
                    let actual = tracker.estimate_at(last_sample).pixels_per_second;
                    let (x, y) = expected[releases];
                    for (actual, expected) in [(f64::from(actual.x), x), (f64::from(actual.y), y)] {
                        assert!(
                            ((actual - expected) / expected).abs() < 0.001,
                            "release {releases}: {actual} vs {expected}",
                        );
                    }
                    releases += 1;
                }
                _ => panic!("unexpected trace phase {phase}"),
            }
        }
        assert_eq!(releases, expected.len());
        assert_eq!(records, 306);
    }

    #[test]
    fn interrupted_motion_uses_only_the_samples_after_the_gap() {
        // Normalized timestamps from Flutter 3.50.0
        // test/gestures/velocity_tracker_data.dart interruptedVelocityEventData.
        let samples = [
            (0, 250.0, 306.0),
            (7, 250.0, 306.0),
            (23, 249.14285278320312_f64, 314.0),
            (30, 247.42857360839844, 319.4285583496094),
            (38, 245.14285278320312, 326.8571472167969),
            (45, 241.7142791748047, 339.4285583496094),
            (93, 238.57142639160156, 355.71429443359375),
            (101, 236.2857208251953, 374.28570556640625),
            (109, 235.14285278320312, 396.5714416503906),
            (117, 236.57142639160156, 421.4285583496094),
            (125, 241.14285278320312, 451.4285583496094),
        ];
        let start = Instant::now();
        let mut tracker = VelocityTracker::new(VelocityTrackerKind::LeastSquares);
        for (time, x, y) in samples {
            tracker.add_position(
                start + Duration::from_millis(time),
                Offset::new(x as f32, y as f32),
            );
        }
        let estimate = tracker.estimate_at(start + Duration::from_millis(140));
        for (actual, expected) in [
            (estimate.pixels_per_second.x, 649.5),
            (estimate.pixels_per_second.y, 3890.3),
        ] {
            assert!(
                ((actual - expected) / expected).abs() < 0.001,
                "{actual} vs {expected}"
            );
        }
        assert_eq!(
            estimate.offset,
            Offset::new(
                samples[10].1 as f32 - samples[6].1 as f32,
                samples[10].2 as f32 - samples[6].2 as f32,
            ),
        );
    }

    #[test]
    fn tracker_with_no_samples_has_zero_velocity() {
        let tracker = VelocityTracker::new(VelocityTrackerKind::LeastSquares);
        assert_eq!(
            tracker.estimate_at(Instant::now()).pixels_per_second,
            Offset::ZERO
        );
    }

    #[test]
    fn all_velocity_strategies_stop_after_samples_go_stale() {
        let start = Instant::now();
        for kind in [
            VelocityTrackerKind::LeastSquares,
            VelocityTrackerKind::IosScrollViewFling,
            VelocityTrackerKind::MacOsScrollViewFling,
        ] {
            let mut tracker = VelocityTracker::new(kind);
            for seconds in 1..=10 {
                tracker.add_position(
                    start + Duration::from_secs(seconds),
                    Offset::new(0.0, -(seconds as f32)),
                );
            }
            let estimate =
                tracker.estimate_at(start + Duration::from_secs(10) + Duration::from_millis(100));
            assert_eq!(estimate.pixels_per_second, Offset::ZERO, "{kind:?}");
        }
    }

    #[test]
    fn ios_consistent_motion_gives_consistent_velocity() {
        let start = Instant::now();
        let mut tracker = VelocityTracker::new(VelocityTrackerKind::IosScrollViewFling);
        for seconds in 1..=10 {
            let time = start + Duration::from_secs(seconds);
            tracker.add_position(time, Offset::new(0.0, -(seconds as f32)));
            let velocity = tracker.estimate_at(time).pixels_per_second;
            if seconds >= 4 {
                assert_eq!(velocity, Offset::new(0.0, -1.0));
            }
        }
    }
}
