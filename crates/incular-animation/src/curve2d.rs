//! Flutter's two-dimensional parametric curves (`Curve2D`,
//! `CatmullRomSpline`) and the timing curve built on them
//! (`CatmullRomCurve`).
//!
//! The arithmetic is done in `f64`, as Dart's is, and the adaptive sampler
//! uses the Dart VM's seeded `Random`, so sample points match Flutter's.

use incular_core::Offset;

/// A point on a [`Curve2D`] and the parameter that produced it: Flutter's
/// `Curve2DSample`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Curve2DSample {
    pub t: f64,
    pub x: f64,
    pub y: f64,
}

impl Curve2DSample {
    /// The sampled point.
    #[must_use]
    pub fn value(&self) -> Offset {
        Offset::new(self.x as f32, self.y as f32)
    }
}

/// A curve from `[0, 1]` to the plane: Flutter's `Curve2D`.
pub trait Curve2D {
    /// The point at `t` in `[0, 1]`, in `f64`.
    fn transform_xy(&self, t: f64) -> (f64, f64);

    /// The point at `t` in `[0, 1]`.
    fn transform(&self, t: f32) -> Offset {
        let (x, y) = self.transform_xy(f64::from(t));
        Offset::new(x as f32, y as f32)
    }

    /// The seed of the sampler's random subdivision points.
    fn sampling_seed(&self) -> i64 {
        0
    }

    /// Flutter `generateSamples`: adaptive samples between `start` and
    /// `end`, denser where the curve bends, ending once each span is flat to
    /// within `tolerance` (Flutter's default is `1e-10`).
    fn generate_samples(&self, start: f64, end: f64, tolerance: f64) -> Vec<Curve2DSample> {
        let mut random = DartRandom::new(self.sampling_seed());
        let sample_at = |t: f64| {
            let (x, y) = self.transform_xy(t);
            Curve2DSample { t, x, y }
        };
        let first = sample_at(start);
        let last = sample_at(end);
        let mut samples = vec![first];
        let force = (first.x - last.x).abs() < tolerance && (first.y - last.y).abs() < tolerance;
        // Flutter recurses; an explicit stack visits the spans in the same
        // order, so the random subdivision points are drawn identically.
        let mut stack = vec![(first, last, force)];
        while let Some((p, q, force)) = stack.pop() {
            let t = p.t + (0.45 + 0.1 * random.next_double()) * (q.t - p.t);
            let r = sample_at(t);
            let (prx, pry) = (p.x - r.x, p.y - r.y);
            let (qrx, qry) = (q.x - r.x, q.y - r.y);
            let z = prx * qry - qrx * pry;
            if !force && z * z < tolerance {
                samples.push(q);
            } else {
                stack.push((r, q, false));
                stack.push((p, r, false));
            }
        }
        samples
    }

    /// Flutter `findInverse`: the parameter whose point has `x`, by
    /// bisection to within `1e-6`.
    fn find_inverse(&self, x: f64) -> f64 {
        let offset_to_origin = |t: f64| x - self.transform_xy(t).0;
        let (mut start, mut end) = (0.0_f64, 1.0_f64);
        let mut mid = 0.0;
        let start_sign = dart_sign(offset_to_origin(start));
        let mut count = 100;
        while (end - start) / 2.0 > 1e-6 && count > 0 {
            mid = (end + start) / 2.0;
            if dart_sign(offset_to_origin(mid)) == start_sign {
                start = mid;
            } else {
                end = mid;
            }
            count -= 1;
        }
        mid
    }
}

fn dart_sign(value: f64) -> f64 {
    if value > 0.0 {
        1.0
    } else if value < 0.0 {
        -1.0
    } else {
        value
    }
}

/// The Dart VM's `Random(seed)`: multiply-with-carry over a Thomas Wang
/// mixed seed.
struct DartRandom {
    state: u64,
}

impl DartRandom {
    fn new(seed: i64) -> Self {
        let mut n = seed as u64;
        n = (!n).wrapping_add(n << 21);
        n ^= n >> 24;
        n = n.wrapping_mul(265);
        n ^= n >> 14;
        n = n.wrapping_mul(21);
        n ^= n >> 28;
        n = n.wrapping_add(n << 31);
        if n == 0 {
            n = 0x5a17;
        }
        let mut random = Self { state: n };
        for _ in 0..4 {
            random.next_state();
        }
        random
    }

    fn next_state(&mut self) {
        const A: u64 = 0xffff_da61;
        let low = self.state & 0xFFFF_FFFF;
        let high = self.state >> 32;
        self.state = A.wrapping_mul(low).wrapping_add(high);
    }

    fn next_power_of_two(&mut self, max: u64) -> u64 {
        self.next_state();
        self.state & 0xFFFF_FFFF & (max - 1)
    }

    fn next_double(&mut self) -> f64 {
        let high = self.next_power_of_two(1 << 26) as f64;
        let low = self.next_power_of_two(1 << 27) as f64;
        (high * (1_u64 << 27) as f64 + low) / (1_u64 << 53) as f64
    }
}

type Point = (f64, f64);

fn sub(a: Point, b: Point) -> Point {
    (a.0 - b.0, a.1 - b.1)
}

fn add(a: Point, b: Point) -> Point {
    (a.0 + b.0, a.1 + b.1)
}

fn scale(a: Point, s: f64) -> Point {
    (a.0 * s, a.1 * s)
}

fn point(offset: Offset) -> Point {
    (f64::from(offset.x), f64::from(offset.y))
}

/// A centripetal Catmull-Rom spline through control points: Flutter's
/// `CatmullRomSpline`.
#[derive(Clone, Debug, PartialEq)]
pub struct CatmullRomSpline {
    /// Each cubic segment's `[a, b, c, d]` for `a t³ + b t² + c t + d`.
    segments: Vec<[Point; 4]>,
}

impl CatmullRomSpline {
    /// A spline through `control_points` (at least four). `tension` in
    /// `[0, 1]` tightens the curve; the handles default to extensions of the
    /// first and last segments.
    ///
    /// # Panics
    ///
    /// Panics with fewer than four control points or a tension outside
    /// `[0, 1]`, as Flutter asserts.
    #[must_use]
    pub fn new(control_points: &[Offset], tension: f32) -> Self {
        Self::with_handles(control_points, tension, None, None)
    }

    /// [`Self::new`] with explicit start and end handles.
    ///
    /// # Panics
    ///
    /// As [`Self::new`].
    #[must_use]
    pub fn with_handles(
        control_points: &[Offset],
        tension: f32,
        start_handle: Option<Offset>,
        end_handle: Option<Offset>,
    ) -> Self {
        assert!(
            (0.0..=1.0).contains(&tension),
            "tension {tension} must be within [0, 1]"
        );
        assert!(
            control_points.len() > 3,
            "There must be at least four control points to create a CatmullRomSpline."
        );
        let points: Vec<Point> = control_points.iter().copied().map(point).collect();
        Self {
            segments: compute_segments(
                &points,
                f64::from(tension),
                start_handle.map(point),
                end_handle.map(point),
            ),
        }
    }

    fn from_points(points: &[Point], tension: f64) -> Self {
        Self {
            segments: compute_segments(points, tension, None, None),
        }
    }
}

fn compute_segments(
    control_points: &[Point],
    tension: f64,
    start_handle: Option<Point>,
    end_handle: Option<Point>,
) -> Vec<[Point; 4]> {
    let count = control_points.len();
    let start_handle =
        start_handle.unwrap_or_else(|| sub(scale(control_points[0], 2.0), control_points[1]));
    let end_handle = end_handle.unwrap_or_else(|| {
        sub(
            scale(control_points[count - 1], 2.0),
            control_points[count - 2],
        )
    });
    let mut all = Vec::with_capacity(count + 2);
    all.push(start_handle);
    all.extend_from_slice(control_points);
    all.push(end_handle);
    // Alpha 0.5 makes the spline centripetal.
    const ALPHA: f64 = 0.5;
    let reverse_tension = 1.0 - tension;
    let distance = |p: Point| p.0.hypot(p.1);
    (0..all.len() - 3)
        .map(|i| {
            let curve = [all[i], all[i + 1], all[i + 2], all[i + 3]];
            let diff10 = sub(curve[1], curve[0]);
            let diff21 = sub(curve[2], curve[1]);
            let diff32 = sub(curve[3], curve[2]);
            let t01 = distance(diff10).powf(ALPHA);
            let t12 = distance(diff21).powf(ALPHA);
            let t23 = distance(diff32).powf(ALPHA);
            let m1 = scale(
                add(
                    diff21,
                    scale(
                        sub(
                            scale(diff10, 1.0 / t01),
                            scale(sub(curve[2], curve[0]), 1.0 / (t01 + t12)),
                        ),
                        t12,
                    ),
                ),
                reverse_tension,
            );
            let m2 = scale(
                add(
                    diff21,
                    scale(
                        sub(
                            scale(diff32, 1.0 / t23),
                            scale(sub(curve[3], curve[1]), 1.0 / (t12 + t23)),
                        ),
                        t12,
                    ),
                ),
                reverse_tension,
            );
            let sum = add(m1, m2);
            [
                add(scale(diff21, -2.0), sum),
                sub(sub(scale(diff21, 3.0), m1), sum),
                m1,
                curve[1],
            ]
        })
        .collect()
}

impl Curve2D for CatmullRomSpline {
    fn transform_xy(&self, t: f64) -> (f64, f64) {
        let length = self.segments.len();
        let (local, index) = if t < 1.0 {
            let position = t * length as f64;
            (position.rem_euclid(1.0), position.floor() as usize)
        } else {
            (1.0, length - 1)
        };
        let [a, b, c, d] = self.segments[index];
        let local2 = local * local;
        add(
            add(scale(a, local2 * local), scale(b, local2)),
            add(scale(c, local), d),
        )
    }

    fn sampling_seed(&self) -> i64 {
        let seed = self.segments[0][1];
        ((seed.0 + seed.1) * 10000.0).round() as i64
    }
}

/// A timing curve through control points between `(0, 0)` and `(1, 1)`:
/// Flutter's `CatmullRomCurve`. Build one with [`crate::Curve::catmull_rom`].
#[derive(Clone, Debug)]
pub struct CatmullRomCurve {
    control_points: Vec<Offset>,
    tension: f32,
    samples: Vec<Curve2DSample>,
}

impl PartialEq for CatmullRomCurve {
    fn eq(&self, other: &Self) -> bool {
        self.control_points == other.control_points && self.tension == other.tension
    }
}

impl CatmullRomCurve {
    /// Precomputes the curve.
    ///
    /// # Panics
    ///
    /// Panics if the control points do not describe a single-valued curve
    /// from `(0, 0)` to `(1, 1)`; see [`Self::validate_control_points`].
    #[must_use]
    pub fn new(control_points: &[Offset], tension: f32) -> Self {
        if let Err(reasons) = Self::validate_control_points(control_points, tension) {
            panic!(
                "control points {control_points:?} could not be validated:\n  {}",
                reasons.join("\n  ")
            );
        }
        let mut points = Vec::with_capacity(control_points.len() + 2);
        points.push((0.0, 0.0));
        points.extend(control_points.iter().copied().map(point));
        points.push((1.0, 1.0));
        let samples = CatmullRomSpline::from_points(&points, f64::from(tension))
            .generate_samples(0.0, 1.0, 1e-12);
        Self {
            control_points: control_points.to_vec(),
            tension,
            samples,
        }
    }

    #[must_use]
    pub fn control_points(&self) -> &[Offset] {
        &self.control_points
    }

    #[must_use]
    pub fn tension(&self) -> f32 {
        self.tension
    }

    /// Flutter `validateControlPoints`: whether `control_points` give a curve
    /// with one y for every x in `[0, 1]`, and if not, why.
    ///
    /// # Errors
    ///
    /// The reasons the points are rejected.
    pub fn validate_control_points(
        control_points: &[Offset],
        tension: f32,
    ) -> Result<(), Vec<String>> {
        let mut reasons = Vec::new();
        if control_points.len() < 2 {
            reasons
                .push("There must be at least two points supplied to create a valid curve.".into());
            return Err(reasons);
        }
        let mut points = Vec::with_capacity(control_points.len() + 4);
        points.push((0.0, 0.0));
        points.extend(control_points.iter().copied().map(point));
        points.push((1.0, 1.0));
        let start_handle = sub(scale(points[0], 2.0), points[1]);
        let n = points.len();
        let end_handle = sub(scale(points[n - 1], 2.0), points[n - 2]);
        points.insert(0, start_handle);
        points.push(end_handle);
        let mut last_x = f64::NEG_INFINITY;
        for (i, p) in points.iter().enumerate() {
            if i > 1 && i < points.len() - 2 && (p.0 <= 0.0 || p.0 >= 1.0) {
                reasons.push(format!(
                    "Control points must have X values between 0.0 and 1.0, exclusive. Point {i} has an x value ({}) which is outside the range.",
                    p.0
                ));
                return Err(reasons);
            }
            if p.0 <= last_x {
                reasons.push(format!(
                    "Each X coordinate must be greater than the preceding X coordinate (i.e. must be monotonically increasing in X). Point {i} has an x value of {}, which is not greater than {last_x}",
                    p.0
                ));
                return Err(reasons);
            }
            last_x = p.0;
        }
        let spline = CatmullRomSpline::from_points(&points, f64::from(tension));
        let start = spline.find_inverse(0.0);
        let end = spline.find_inverse(1.0);
        let samples = spline.generate_samples(start, end, 1e-10);
        const TOLERANCE: f64 = 1e-3;
        let first = samples[0];
        let last = samples[samples.len() - 1];
        if first.y.abs() > TOLERANCE || (1.0 - last.y).abs() > TOLERANCE {
            reasons.push(format!(
                "The curve has more than one Y value at X = {}. Try moving some control points further away from this value of X, or increasing the tension.",
                first.x
            ));
        }
        let mut last_x = f64::NEG_INFINITY;
        for sample in &samples {
            if sample.t >= start && sample.t <= end && (sample.x < -1e-3 || sample.x > 1.0 + 1e-3) {
                reasons.push(format!(
                    "The resulting curve has an X value ({}) which is outside the range [0.0, 1.0], inclusive.",
                    sample.x
                ));
            }
            if sample.x < last_x {
                reasons.push(format!(
                    "The curve has more than one Y value at x = {}. Try moving some control points further apart in X, or increasing the tension.",
                    sample.x
                ));
            }
            last_x = sample.x;
        }
        if reasons.is_empty() {
            Ok(())
        } else {
            Err(reasons)
        }
    }

    /// Flutter `transformInternal`: interpolates between the precomputed
    /// samples around `t`.
    #[must_use]
    pub fn transform(&self, t: f32) -> f32 {
        let t = f64::from(t);
        let samples = &self.samples;
        let (mut start, mut end) = (0, samples.len() - 1);
        while end - start > 1 {
            let mid = (end + start) / 2;
            if t >= samples[mid].x {
                start = mid;
            } else {
                end = mid;
            }
        }
        let (a, b) = (samples[start], samples[end]);
        let t2 = (t - a.x) / (b.x - a.x);
        (a.y + (b.y - a.y) * t2) as f32
    }
}
