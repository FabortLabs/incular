use crate::display_list::ImageSampling;
use incular_core::finite_or_zero;
use incular_core::{Color, IdGenerator, Lerp, Offset, Rect};
use std::sync::Arc;

static NEXT_GRADIENT_ID: IdGenerator = IdGenerator::new();

fn next_gradient_id() -> GradientId {
    GradientId(NEXT_GRADIENT_ID.next())
}

/// Stable identity for immutable normalized gradient stop data.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct GradientId(u64);

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GradientStop {
    pub offset: f32,
    pub color: Color,
}
#[derive(Clone, Debug)]
pub struct GradientStops {
    id: GradientId,
    stops: Arc<[GradientStop]>,
    explicit_positions: bool,
}
impl PartialEq for GradientStops {
    fn eq(&self, other: &Self) -> bool {
        self.stops == other.stops
    }
}

/// Invalid colors and stop positions supplied separately for a gradient.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GradientStopsError {
    /// Computing implied stops requires at least two colors.
    TooFewColors { colors: usize },
    /// An explicit stop must be supplied for each color.
    LengthMismatch { colors: usize, stops: usize },
}

impl std::fmt::Display for GradientStopsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Self::TooFewColors { colors } => {
                write!(
                    f,
                    "implied gradient stops need at least two colors (got {colors})"
                )
            }
            Self::LengthMismatch { colors, stops } => {
                write!(
                    f,
                    "gradient colors and stops must have equal length (got {colors} colors and {stops} stops)"
                )
            }
        }
    }
}

impl std::error::Error for GradientStopsError {}

/// Samples normalized stops using pad spread and premultiplied interpolation
/// in the encoded-sRGB working space (see [`crate::working_rgba`]). At a duplicate offset the last duplicate wins, allowing a
/// hard transition without division by zero.
#[must_use]
pub fn sample_gradient_stops(stops: &GradientStops, t: f32) -> [f32; 4] {
    let stops = stops.as_slice();
    let t = finite_or_zero(t);
    if t <= stops[0].offset {
        return premultiplied(stops[0].color);
    }
    let last = stops.last().expect("normalized stops are non-empty");
    if t >= last.offset {
        return premultiplied(last.color);
    }
    let mut left = 0;
    for index in 1..stops.len() {
        if stops[index].offset <= t {
            left = index;
        } else {
            let a = stops[left];
            let b = stops[index];
            let amount = ((t - a.offset) / (b.offset - a.offset)).clamp(0., 1.);
            let a = premultiplied(a.color);
            let b = premultiplied(b.color);
            return [
                a[0] + (b[0] - a[0]) * amount,
                a[1] + (b[1] - a[1]) * amount,
                a[2] + (b[2] - a[2]) * amount,
                a[3] + (b[3] - a[3]) * amount,
            ];
        }
    }
    premultiplied(last.color)
}
fn premultiplied(color: Color) -> [f32; 4] {
    crate::premultiplied_working_rgba(color)
}
/// Pad-spread linear parameter in local logical coordinates. A zero-length
/// axis deterministically chooses the last stop (`t = 1`).
#[must_use]
pub fn linear_gradient_t(start: Offset, end: Offset, point: Offset) -> f32 {
    let dx = end.x - start.x;
    let dy = end.y - start.y;
    let length_squared = dx * dx + dy * dy;
    if !length_squared.is_finite() || length_squared <= f32::EPSILON {
        return 1.;
    }
    (((point.x - start.x) * dx + (point.y - start.y) * dy) / length_squared).clamp(0., 1.)
}
/// Pad-spread radial parameter in local logical coordinates. A non-positive
/// radius deterministically chooses the last stop (`t = 1`).
#[must_use]
pub fn radial_gradient_t(center: Offset, radius: f32, point: Offset) -> f32 {
    if !radius.is_finite() || radius <= 0. {
        return 1.;
    }
    (((point.x - center.x).hypot(point.y - center.y)) / radius).clamp(0., 1.)
}
impl GradientStops {
    /// Stops are clamped to 0..=1 and stable-sorted. Empty becomes transparent;
    /// one stop is duplicated so shader sampling is always defined.
    #[must_use]
    pub fn new(mut stops: Vec<GradientStop>) -> Self {
        for s in &mut stops {
            s.offset = if s.offset.is_finite() {
                s.offset.clamp(0., 1.)
            } else {
                0.
            };
        }
        stops.sort_by(|a, b| a.offset.total_cmp(&b.offset));
        if stops.is_empty() {
            stops.push(GradientStop {
                offset: 0.,
                color: Color::TRANSPARENT,
            });
        }
        if stops.len() == 1 {
            stops.push(GradientStop {
                offset: 1.,
                color: stops[0].color,
            });
        }
        Self {
            id: next_gradient_id(),
            stops: stops.into(),
            explicit_positions: true,
        }
    }
    /// Pairs colors with optional stop positions, rejecting a mismatched list.
    /// If the positions are absent, colors are distributed evenly over the unit interval.
    pub fn from_colors(
        colors: &[Color],
        positions: Option<&[f32]>,
    ) -> Result<Self, GradientStopsError> {
        if let Some(positions) = positions {
            if colors.len() != positions.len() {
                return Err(GradientStopsError::LengthMismatch {
                    colors: colors.len(),
                    stops: positions.len(),
                });
            }
            Ok(Self::new(
                colors
                    .iter()
                    .zip(positions)
                    .map(|(&color, &offset)| GradientStop { offset, color })
                    .collect(),
            ))
        } else if colors.len() < 2 {
            Err(GradientStopsError::TooFewColors {
                colors: colors.len(),
            })
        } else {
            Ok(Self::even(colors))
        }
    }

    /// Builds implied stops spaced evenly across the unit interval.
    #[must_use]
    pub fn even(colors: &[Color]) -> Self {
        let denominator = colors.len().saturating_sub(1).max(1) as f32;
        let mut value = Self::new(
            colors
                .iter()
                .enumerate()
                .map(|(i, &color)| GradientStop {
                    offset: i as f32 / denominator,
                    color,
                })
                .collect(),
        );
        value.explicit_positions = false;
        value
    }

    /// Whether the stop positions were supplied rather than inferred.
    /// This affects descriptions but not renderer equality or cache keys.
    #[must_use]
    pub const fn has_explicit_positions(&self) -> bool {
        self.explicit_positions
    }

    /// Fades opacity without changing stop positions or RGB.
    #[must_use]
    pub fn scale_opacity(&self, factor: f32) -> Self {
        if factor == 1.0 {
            return self.clone();
        }
        self.with_colors(|color| Color {
            alpha: quantize(f64::from(color.alpha) * f64::from(factor)),
            ..color
        })
    }

    /// Replaces opacity rather than multiplying the existing alpha.
    #[must_use]
    pub fn with_opacity(&self, opacity: f32) -> Self {
        let alpha = quantize(f64::from(opacity) * 255.);
        self.with_colors(|color| Color { alpha, ..color })
    }

    /// Replaces all colors while preserving stop geometry.
    #[must_use]
    pub fn with_color(&self, color: Color) -> Self {
        self.with_colors(|_| color)
    }

    fn with_colors(&self, mut map: impl FnMut(Color) -> Color) -> Self {
        Self {
            id: next_gradient_id(),
            stops: self
                .stops
                .iter()
                .map(|stop| GradientStop {
                    offset: stop.offset,
                    color: map(stop.color),
                })
                .collect(),
            explicit_positions: self.explicit_positions,
        }
    }

    /// Interpolates on the sorted union of stop offsets. Color samples
    /// are straight encoded-sRGB here; premultiplication belongs to
    /// the renderer's later spatial sampling step.
    #[must_use]
    pub fn interpolate(&self, other: &Self, t: f32) -> Self {
        if self == other
            && self
                .stops
                .windows(2)
                .all(|pair| pair[0].offset != pair[1].offset)
        {
            return self.clone();
        }
        let (a, b) = (self.as_slice(), other.as_slice());
        let mut indices = (0, 0);
        let mut cursors = (0, 0);
        let mut output = Vec::with_capacity(a.len() + b.len());
        while indices.0 < a.len() || indices.1 < b.len() {
            let u = a.get(indices.0).map_or(f32::INFINITY, |stop| stop.offset);
            let v = b.get(indices.1).map_or(f32::INFINITY, |stop| stop.offset);
            let offset = u.min(v);
            while a.get(indices.0).is_some_and(|stop| stop.offset == offset) {
                indices.0 += 1;
            }
            while b.get(indices.1).is_some_and(|stop| stop.offset == offset) {
                indices.1 += 1;
            }
            let a = straight_sample(a, offset, &mut cursors.0);
            let b = straight_sample(b, offset, &mut cursors.1);
            let channels = mix_channels(a, b, f64::from(t)).map(quantize);
            output.push(GradientStop {
                offset,
                color: Color::rgba(channels[0], channels[1], channels[2], channels[3]),
            });
        }
        // All stops of a hard transition can share a single offset.
        if output.len() == 1 {
            output.push(output[0]);
        }
        Self {
            id: next_gradient_id(),
            stops: output.into(),
            explicit_positions: true,
        }
    }

    #[must_use]
    pub fn as_slice(&self) -> &[GradientStop] {
        &self.stops
    }
    /// Identity is computed when normalized stops are constructed, never while
    /// rendering. Clones intentionally retain the same cached GPU resource.
    #[must_use]
    pub const fn id(&self) -> GradientId {
        self.id
    }
}
fn quantize(channel: f64) -> u8 {
    channel.round().clamp(0., 255.) as u8
}

fn mix_channels(a: [f64; 4], b: [f64; 4], t: f64) -> [f64; 4] {
    std::array::from_fn(|i| a[i] + (b[i] - a[i]) * t)
}

fn straight_sample(stops: &[GradientStop], t: f32, cursor: &mut usize) -> [f64; 4] {
    let channels = |stop: GradientStop| {
        let Color {
            red,
            green,
            blue,
            alpha,
        } = stop.color;
        [red, green, blue, alpha].map(f64::from)
    };
    if t <= stops[0].offset {
        return channels(stops[0]);
    }
    while stops.get(*cursor + 1).is_some_and(|stop| stop.offset <= t) {
        *cursor += 1;
    }
    let a = stops[*cursor];
    let Some(&b) = stops.get(*cursor + 1) else {
        return channels(a);
    };
    let fraction =
        (f64::from(t) - f64::from(a.offset)) / (f64::from(b.offset) - f64::from(a.offset));
    mix_channels(channels(a), channels(b), fraction)
}

impl GradientId {
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}
/// Flutter `TileMode`: how a gradient paints outside its `0.0..=1.0`
/// parameter range.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TileMode {
    /// Extends the edge colors (Flutter's default).
    #[default]
    Clamp,
    /// Repeats the gradient.
    Repeated,
    /// Repeats the gradient, reversing every other repetition.
    Mirror,
    /// Paints transparent black outside the gradient.
    Decal,
}

impl TileMode {
    /// The stop parameter sampled for a raw gradient parameter `t`, or
    /// `None` where [`Self::Decal`] paints nothing.
    #[must_use]
    pub fn apply(self, t: f32) -> Option<f32> {
        let t = finite_or_zero(t);
        match self {
            Self::Clamp => Some(t.clamp(0., 1.)),
            Self::Repeated => Some(t - t.floor()),
            Self::Mirror => {
                let period = t * 0.5 - (t * 0.5).floor();
                Some(1. - (period * 2. - 1.).abs())
            }
            Self::Decal => (0. ..=1.).contains(&t).then_some(t),
        }
    }

    /// The code the GPU shaders read.
    #[must_use]
    pub const fn shader_code(self) -> f32 {
        match self {
            Self::Clamp => 0.,
            Self::Repeated => 1.,
            Self::Mirror => 2.,
            Self::Decal => 3.,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct LinearGradient {
    pub start: Offset,
    pub end: Offset,
    pub stops: GradientStops,
    /// Flutter `tileMode`, default [`TileMode::Clamp`].
    pub tile_mode: TileMode,
}
#[derive(Clone, Debug, PartialEq)]
pub struct RadialGradient {
    pub center: Offset,
    pub radius: f32,
    pub stops: GradientStops,
    /// Flutter `tileMode`, default [`TileMode::Clamp`].
    pub tile_mode: TileMode,
    /// Flutter `focal`: where the gradient's `0.0` circle is centered. With
    /// `None` it is `center`, a plain radial gradient.
    pub focal: Option<Offset>,
    /// Flutter `focalRadius` (default `0.0`): the radius of the `0.0`
    /// circle around [`Self::focal`].
    pub focal_radius: f32,
}
#[derive(Clone, Debug, PartialEq)]
pub struct SweepGradient {
    pub center: Offset,
    pub start_angle: f32,
    /// Explicit end of the angular sector. None retains the existing
    /// full-turn brush parameter wrapped relative to start_angle.
    pub end_angle: Option<f32>,
    /// Physical rotation of an explicit sector's branch cut.
    pub rotation: f32,
    pub stops: GradientStops,
    /// Flutter `tileMode`, default [`TileMode::Clamp`].
    pub tile_mode: TileMode,
}

impl LinearGradient {
    /// A clamped linear gradient from `start` to `end`.
    #[must_use]
    pub fn new(start: Offset, end: Offset, stops: GradientStops) -> Self {
        Self {
            start,
            end,
            stops,
            tile_mode: TileMode::Clamp,
        }
    }
}

impl RadialGradient {
    /// A clamped radial gradient without a focal point.
    #[must_use]
    pub fn new(center: Offset, radius: f32, stops: GradientStops) -> Self {
        Self {
            center,
            radius,
            stops,
            tile_mode: TileMode::Clamp,
            focal: None,
            focal_radius: 0.,
        }
    }

    /// Whether this is Flutter's two-point conical form: a focal point other
    /// than the center, or a focal radius.
    #[must_use]
    pub fn is_focal(&self) -> bool {
        self.focal.is_some_and(|focal| focal != self.center) || self.focal_radius != 0.
    }
}

impl SweepGradient {
    /// A clamped sweep gradient starting at `start_angle`.
    #[must_use]
    pub fn new(center: Offset, start_angle: f32, stops: GradientStops) -> Self {
        Self {
            center,
            start_angle,
            end_angle: None,
            rotation: 0.,
            stops,
            tile_mode: TileMode::Clamp,
        }
    }
    /// Selects an explicit clockwise sector instead of a full turn.
    #[must_use]
    pub fn with_end_angle(mut self, end: f32) -> Self {
        self.end_angle = Some(end.max(self.start_angle));
        self
    }

    /// Untiled parameter for renderer-neutral inspection or sampling.
    #[must_use]
    pub fn parameter(&self, point: Offset) -> f32 {
        use std::f32::consts::TAU;

        let Some(end) = self.end_angle else {
            return sweep_gradient_t(self.center, self.start_angle, point);
        };
        let angle = ((point.y - self.center.y).atan2(point.x - self.center.x) - self.rotation)
            .rem_euclid(TAU);
        let span = end - self.start_angle;
        if span > 0. {
            (angle - self.start_angle) / span
        } else if angle < self.start_angle {
            -1.
        } else {
            2.
        }
    }
}

/// The raw parameter of Flutter's focal radial gradient (Skia's two-point
/// conical gradient from the circle `(focal, focal_radius)` at `0.0` to
/// `(center, radius)` at `1.0`): the largest `t` whose interpolated circle,
/// with a non-negative radius, passes through `point`. `None` where no such
/// circle exists, which paints nothing.
#[must_use]
pub fn two_point_conical_gradient_t(
    focal: Offset,
    focal_radius: f32,
    center: Offset,
    radius: f32,
    point: Offset,
) -> Option<f32> {
    let cd = Offset::new(center.x - focal.x, center.y - focal.y);
    let dr = radius - focal_radius;
    let pd = Offset::new(point.x - focal.x, point.y - focal.y);
    let a = cd.x * cd.x + cd.y * cd.y - dr * dr;
    let b = pd.x * cd.x + pd.y * cd.y + focal_radius * dr;
    let c = pd.x * pd.x + pd.y * pd.y - focal_radius * focal_radius;
    let radius_at = |t: f32| focal_radius + t * dr;
    if a.abs() <= f32::EPSILON {
        // The circles touch: the quadratic degenerates to `-2bt + c = 0`.
        if b == 0. {
            return None;
        }
        let t = c / (2. * b);
        return (radius_at(t) >= 0.).then_some(t);
    }
    let discriminant = b * b - a * c;
    if discriminant < 0. {
        return None;
    }
    let root = discriminant.sqrt();
    let (high, low) = {
        let first = (b + root) / a;
        let second = (b - root) / a;
        (first.max(second), first.min(second))
    };
    if radius_at(high) >= 0. {
        Some(high)
    } else if radius_at(low) >= 0. {
        Some(low)
    } else {
        None
    }
}
#[derive(Clone, Debug, PartialEq)]
pub enum Brush {
    Solid(Color),
    LinearGradient(LinearGradient),
    RadialGradient(RadialGradient),
    SweepGradient(SweepGradient),
}

/// A renderer-neutral shader value.
///
/// Incular's gradients are already immutable, cacheable shader descriptions;
/// keeping this alias means widgets and text do not need a second shader
/// hierarchy that would eventually drift from the renderer's brush model.
pub type Shader = Brush;

/// Image filtering policy matching Flutter's public quality vocabulary.
///
/// The current display-list backend exposes nearest-neighbour and linear
/// sampling. `Low`, `Medium`, and `High` intentionally lower to linear until
/// a backend grows a more expensive filter; callers still retain a stable,
/// renderer-independent policy value.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum FilterQuality {
    None,
    #[default]
    Low,
    Medium,
    High,
}

impl FilterQuality {
    #[must_use]
    pub const fn sampling(self) -> ImageSampling {
        match self {
            Self::None => ImageSampling::Nearest,
            Self::Low | Self::Medium | Self::High => ImageSampling::Linear,
        }
    }

    /// Sampling for a transformed, retained raster source. Medium differs
    /// from Low when an output texel covers several source texels.
    #[must_use]
    pub const fn matrix_sampling(self) -> ImageSampling {
        match self {
            Self::Medium => ImageSampling::LinearMipmap,
            quality => quality.sampling(),
        }
    }
}

impl From<FilterQuality> for ImageSampling {
    fn from(value: FilterQuality) -> Self {
        value.sampling()
    }
}
impl Brush {
    /// Interpolates shader geometry in absolute coordinates for two brushes
    /// of the same kind, including resampling their merged gradient stops.
    /// Different brush families need a policy from the caller.
    #[must_use]
    pub fn interpolate_same_kind(&self, other: &Self, t: f32) -> Option<Self> {
        if std::mem::discriminant(self) != std::mem::discriminant(other) {
            return None;
        }
        if std::ptr::eq(self, other) || t == 0. {
            return Some(self.clone());
        }
        if t == 1. {
            return Some(other.clone());
        }
        let mix = |a: f32, b: f32| a + (b - a) * t;
        let point = |a: Offset, b: Offset| Offset::new(mix(a.x, b.x), mix(a.y, b.y));
        let tile = |a, b| if t < 0.5 { a } else { b };
        Some(match (self, other) {
            (Self::Solid(a), Self::Solid(b)) => Self::Solid(a.lerp(b, t)),
            (Self::LinearGradient(a), Self::LinearGradient(b)) => Self::LinearGradient(LinearGradient {
                start: point(a.start, b.start),
                end: point(a.end, b.end),
                stops: a.stops.interpolate(&b.stops, t),
                tile_mode: tile(a.tile_mode, b.tile_mode),
            }),
            (Self::RadialGradient(a), Self::RadialGradient(b)) => Self::RadialGradient(RadialGradient {
                center: point(a.center, b.center),
                radius: mix(a.radius, b.radius).max(0.),
                stops: a.stops.interpolate(&b.stops, t),
                tile_mode: tile(a.tile_mode, b.tile_mode),
                focal: (a.focal.is_some() || b.focal.is_some()).then(|| {
                    point(a.focal.unwrap_or(a.center), b.focal.unwrap_or(b.center))
                }),
                focal_radius: mix(a.focal_radius, b.focal_radius).max(0.),
            }),
            (Self::SweepGradient(a), Self::SweepGradient(b)) => {
                let (start_angle, end_angle, rotation) = match (a.end_angle, b.end_angle) {
                    (None, None) => (mix(a.start_angle, b.start_angle), None, 0.),
                    (Some(end_a), Some(end_b)) => {
                        let start = mix(a.start_angle, b.start_angle).max(0.);
                        let rotation = if t < 0.5 { a.rotation } else { b.rotation };
                        (start, Some(mix(end_a, end_b).max(start)), rotation)
                    }
                    _ => {
                        // An implicit full turn wraps at its own start. Its
                        // exactly equivalent explicit representation starts
                        // at zero with that start in the branch rotation.
                        let sector = |gradient: &SweepGradient| {
                            gradient.end_angle.map_or(
                                (0., std::f32::consts::TAU, gradient.start_angle),
                                |end| (gradient.start_angle, end, gradient.rotation),
                            )
                        };
                        let (start_a, end_a, rotation_a) = sector(a);
                        let (start_b, end_b, rotation_b) = sector(b);
                        let start = mix(start_a, start_b).max(0.);
                        (start, Some(mix(end_a, end_b).max(start)), mix(rotation_a, rotation_b))
                    }
                };
                Self::SweepGradient(SweepGradient {
                    center: point(a.center, b.center),
                    start_angle,
                    end_angle,
                    rotation,
                    stops: a.stops.interpolate(&b.stops, t),
                    tile_mode: tile(a.tile_mode, b.tile_mode),
                })
            }
            _ => unreachable!("brush discriminants were checked"),
        })
    }

    /// Applies Flutter's `GradientRotation(radians)` for a shader created
    /// over `bounds`: the gradient field rotates by `radians` (clockwise
    /// in Incular's y-down space, as in Flutter) about the centre of
    /// `bounds` (`gradient.dart` `GradientRotation.transform`).
    ///
    /// Rotation is exact for every brush: linear endpoints, the radial
    /// centre and focal point, and the sweep centre rotate, and a sweep's
    /// branch cut rotates by `radians`. Solid brushes are unchanged.
    /// General `GradientTransform` matrices (scale, skew) are not
    /// representable by the brush value.
    #[must_use]
    pub fn rotated(&self, radians: f32, bounds: Rect) -> Self {
        if !radians.is_finite() || radians == 0. {
            return self.clone();
        }
        let center = Offset::new(
            bounds.origin.x + bounds.size.width * 0.5,
            bounds.origin.y + bounds.size.height * 0.5,
        );
        let (sin, cos) = radians.sin_cos();
        let rotate = |point: Offset| {
            let (x, y) = (point.x - center.x, point.y - center.y);
            Offset::new(center.x + x * cos - y * sin, center.y + x * sin + y * cos)
        };
        match self {
            Self::Solid(color) => Self::Solid(*color),
            Self::LinearGradient(gradient) => Self::LinearGradient(LinearGradient {
                start: rotate(gradient.start),
                end: rotate(gradient.end),
                ..gradient.clone()
            }),
            Self::RadialGradient(gradient) => Self::RadialGradient(RadialGradient {
                center: rotate(gradient.center),
                focal: gradient.focal.map(rotate),
                ..gradient.clone()
            }),
            Self::SweepGradient(gradient) => {
                let mut rotated = gradient.clone();
                rotated.center = rotate(gradient.center);
                if gradient.end_angle.is_some() {
                    rotated.rotation += radians;
                } else {
                    rotated.start_angle += radians;
                }
                Self::SweepGradient(rotated)
            }
        }
    }
}
impl From<Color> for Brush {
    fn from(color: Color) -> Self {
        Self::Solid(color)
    }
}
impl From<LinearGradient> for Brush {
    fn from(gradient: LinearGradient) -> Self {
        Self::LinearGradient(gradient)
    }
}
impl From<RadialGradient> for Brush {
    fn from(gradient: RadialGradient) -> Self {
        Self::RadialGradient(gradient)
    }
}
impl From<SweepGradient> for Brush {
    fn from(gradient: SweepGradient) -> Self {
        Self::SweepGradient(gradient)
    }
}
#[must_use]
pub fn sweep_gradient_t(center: Offset, start_angle: f32, point: Offset) -> f32 {
    ((point.y - center.y).atan2(point.x - center.x) - start_angle).rem_euclid(std::f32::consts::TAU)
        / std::f32::consts::TAU
}
