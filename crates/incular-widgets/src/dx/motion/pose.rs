//! [`Pose`]: a target look made only of compositor properties.

use incular_animation::{Channel, MotionValue, Transition};
use incular_config::Alignment;
use incular_core::{Color, Offset};
use incular_rendering::Shadow;

/// One animatable compositor property of a [`Pose`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(u8)]
pub enum Prop {
    Opacity,
    /// Horizontal translation in logical pixels.
    X,
    /// Vertical translation in logical pixels.
    Y,
    /// Horizontal translation as a fraction of the element's width.
    SlideX,
    /// Vertical translation as a fraction of the element's height.
    SlideY,
    /// Uniform scale, multiplied with `ScaleX` / `ScaleY`.
    Scale,
    ScaleX,
    ScaleY,
    /// Clockwise rotation in turns.
    Rotate,
    SkewX,
    SkewY,
    /// Gaussian blur sigma.
    Blur,
    /// Brightness multiplier (1 is unchanged).
    Brightness,
    /// Saturation (1 is unchanged, 0 is gray).
    Saturate,
    /// Grayscale amount (0 is unchanged, 1 is gray).
    Grayscale,
    /// How strongly the tint color is applied (0 to 1).
    Tint,
    ShadowX,
    ShadowY,
    ShadowBlur,
    /// How strongly the shadow color is applied (0 to 1).
    Shadow,
    /// Clip inset from each edge, as a fraction of the size.
    RevealTop,
    RevealRight,
    RevealBottom,
    RevealLeft,
    /// Circular reveal radius as a fraction of the half-diagonal (1 covers
    /// the whole box). Negative means no circular clip.
    RevealCircle,
}

/// The number of [`Prop`]s.
pub const PROP_COUNT: usize = Prop::RevealCircle as usize + 1;

impl Prop {
    /// Every property, in declaration order.
    pub const ALL: [Prop; PROP_COUNT] = [
        Prop::Opacity,
        Prop::X,
        Prop::Y,
        Prop::SlideX,
        Prop::SlideY,
        Prop::Scale,
        Prop::ScaleX,
        Prop::ScaleY,
        Prop::Rotate,
        Prop::SkewX,
        Prop::SkewY,
        Prop::Blur,
        Prop::Brightness,
        Prop::Saturate,
        Prop::Grayscale,
        Prop::Tint,
        Prop::ShadowX,
        Prop::ShadowY,
        Prop::ShadowBlur,
        Prop::Shadow,
        Prop::RevealTop,
        Prop::RevealRight,
        Prop::RevealBottom,
        Prop::RevealLeft,
        Prop::RevealCircle,
    ];

    /// The value that leaves the element unchanged.
    #[must_use]
    pub const fn identity(self) -> f32 {
        match self {
            Prop::Opacity
            | Prop::Scale
            | Prop::ScaleX
            | Prop::ScaleY
            | Prop::Brightness
            | Prop::Saturate => 1.,
            Prop::RevealCircle => -1.,
            _ => 0.,
        }
    }

    /// The transition channel this property animates on.
    #[must_use]
    pub const fn channel(self) -> Channel {
        match self {
            Prop::Opacity => Channel::Opacity,
            Prop::Brightness | Prop::Saturate | Prop::Grayscale | Prop::Tint | Prop::Shadow => {
                Channel::Color
            }
            _ => Channel::Geometry,
        }
    }

    /// The compositor stage the property lives on.
    #[must_use]
    pub const fn stage(self) -> Stage {
        match self {
            Prop::Opacity => Stage::Opacity,
            Prop::Blur => Stage::Blur,
            Prop::Brightness | Prop::Saturate | Prop::Grayscale | Prop::Tint => Stage::Color,
            Prop::ShadowX | Prop::ShadowY | Prop::ShadowBlur | Prop::Shadow => Stage::Shadow,
            Prop::RevealTop
            | Prop::RevealRight
            | Prop::RevealBottom
            | Prop::RevealLeft
            | Prop::RevealCircle => Stage::Clip,
            _ => Stage::Transform,
        }
    }

    pub(crate) const fn index(self) -> usize {
        self as usize
    }

    /// How values from separate channels (pose, timeline, drag, layout)
    /// combine for this property.
    #[must_use]
    pub const fn combine(self) -> Combine {
        match self {
            Prop::Opacity | Prop::Scale | Prop::ScaleX | Prop::ScaleY => Combine::Multiply,
            Prop::X | Prop::Y | Prop::SlideX | Prop::SlideY | Prop::Rotate => Combine::Add,
            _ => Combine::Override,
        }
    }
}

/// How two channels' values of one property combine.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Combine {
    Add,
    Multiply,
    /// The later channel's non-identity value wins.
    Override,
}

/// The compositor layer a property is applied on, outermost first.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Stage {
    Transform,
    Opacity,
    Shadow,
    Color,
    Blur,
    Clip,
}

/// A pose property's value: a constant, or a live [`MotionValue`] the
/// compositor follows without rebuilding.
#[derive(Clone, Debug, PartialEq)]
pub enum PoseValue {
    Const(f32),
    Bound(MotionValue),
}

impl From<f32> for PoseValue {
    fn from(value: f32) -> Self {
        Self::Const(value)
    }
}

impl From<&MotionValue> for PoseValue {
    fn from(value: &MotionValue) -> Self {
        Self::Bound(value.clone())
    }
}

impl From<MotionValue> for PoseValue {
    fn from(value: MotionValue) -> Self {
        Self::Bound(value)
    }
}

impl PoseValue {
    #[must_use]
    pub fn get(&self) -> f32 {
        match self {
            Self::Const(value) => *value,
            Self::Bound(value) => value.get(),
        }
    }
}

/// A target look, made only of compositor properties: transforms, opacity,
/// color filters, blur, shadow and clip reveals. Applying a pose never
/// relayouts or repaints the element's content.
///
/// Pose builders are plain functions, so presets are `fn`s:
///
/// ```ignore
/// fn lift(p: Pose) -> Pose { p.y(-4.0).scale(1.02) }
/// card.motion(|m| m.hovered(lift))
/// ```
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Pose {
    values: [Option<PoseValue>; PROP_COUNT],
    pub(crate) transition: Option<Transition>,
    pub(crate) origin: Option<Alignment>,
    pub(crate) tint_color: Option<Color>,
    pub(crate) shadow_color: Option<Color>,
    pub(crate) reveal_center: Option<Alignment>,
}

macro_rules! pose_setters {
    ($($(#[$doc:meta])* $name:ident => $prop:ident;)*) => {
        $(
            $(#[$doc])*
            #[must_use]
            pub fn $name(self, value: impl Into<PoseValue>) -> Self {
                self.set(Prop::$prop, value)
            }
        )*
    };
}

impl Pose {
    /// An empty pose: it sets nothing.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets one property.
    #[must_use]
    pub fn set(mut self, prop: Prop, value: impl Into<PoseValue>) -> Self {
        let value = match value.into() {
            PoseValue::Const(v) if !v.is_finite() => PoseValue::Const(prop.identity()),
            value => value,
        };
        self.values[prop.index()] = Some(value);
        self
    }

    /// The value this pose gives `prop`, if it sets it.
    #[must_use]
    pub fn get(&self, prop: Prop) -> Option<&PoseValue> {
        self.values[prop.index()].as_ref()
    }

    /// Whether this pose sets `prop`.
    #[must_use]
    pub fn sets(&self, prop: Prop) -> bool {
        self.values[prop.index()].is_some()
    }

    /// Whether this pose sets nothing.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.values.iter().all(Option::is_none)
    }

    /// The properties this pose sets.
    pub fn props(&self) -> impl Iterator<Item = Prop> + '_ {
        Prop::ALL.into_iter().filter(|prop| self.sets(*prop))
    }

    /// Every property of `other` overrides this pose's.
    #[must_use]
    pub fn merge(mut self, other: &Pose) -> Self {
        for prop in other.props() {
            self.values[prop.index()] = other.values[prop.index()].clone();
        }
        if other.transition.is_some() {
            self.transition.clone_from(&other.transition);
        }
        self.origin = other.origin.or(self.origin);
        self.tint_color = other.tint_color.or(self.tint_color);
        self.shadow_color = other.shadow_color.or(self.shadow_color);
        self.reveal_center = other.reveal_center.or(self.reveal_center);
        self
    }

    pose_setters! {
        /// Alpha from 0 (invisible) to 1.
        opacity => Opacity;
        /// Horizontal offset in logical pixels.
        x => X;
        /// Vertical offset in logical pixels.
        y => Y;
        /// Horizontal offset as a fraction of the element's width.
        slide_x => SlideX;
        /// Vertical offset as a fraction of the element's height.
        slide_y => SlideY;
        /// Uniform scale about the origin (center by default).
        scale => Scale;
        scale_x => ScaleX;
        scale_y => ScaleY;
        /// Clockwise rotation in turns (1 is a full revolution).
        rotate => Rotate;
        skew_x => SkewX;
        skew_y => SkewY;
        /// Gaussian blur sigma. Blur is the most expensive pose property.
        blur => Blur;
        /// Brightness multiplier: 1 is unchanged, above 1 is brighter.
        brightness => Brightness;
        /// Saturation: 1 is unchanged, 0 is gray, above 1 is vivid.
        saturate => Saturate;
        /// Grayscale amount: 0 is unchanged, 1 is fully gray.
        grayscale => Grayscale;
    }

    /// Translation in logical pixels.
    #[must_use]
    pub fn offset(self, offset: Offset) -> Self {
        self.x(offset.x).y(offset.y)
    }

    /// The pivot for scale, skew and rotation (default center).
    #[must_use]
    pub fn origin(mut self, origin: Alignment) -> Self {
        self.origin = Some(origin);
        self
    }

    /// Tints toward `color` by `amount` (0 to 1).
    #[must_use]
    pub fn tint(mut self, color: Color, amount: impl Into<PoseValue>) -> Self {
        self.tint_color = Some(color);
        self.set(Prop::Tint, amount)
    }

    /// A drop shadow following the element's painted shape.
    #[must_use]
    pub fn shadow(mut self, shadow: Shadow) -> Self {
        self.shadow_color = Some(shadow.color);
        self.set(Prop::ShadowX, shadow.offset.x)
            .set(Prop::ShadowY, shadow.offset.y)
            .set(Prop::ShadowBlur, shadow.blur_radius)
            .set(Prop::Shadow, 1.)
    }

    /// A soft black shadow below the element, as a hover "lift".
    #[must_use]
    pub fn lift(self, blur: f32) -> Self {
        let blur = if blur.is_finite() { blur.max(0.) } else { 0. };
        self.shadow(Shadow::new(
            Color::rgba(0, 0, 0, 64),
            Offset::new(0., blur / 3.),
            blur,
        ))
    }

    /// Clips to the box inset by fractions of its size from each edge.
    #[must_use]
    pub fn reveal_inset(self, top: f32, right: f32, bottom: f32, left: f32) -> Self {
        self.set(Prop::RevealTop, top)
            .set(Prop::RevealRight, right)
            .set(Prop::RevealBottom, bottom)
            .set(Prop::RevealLeft, left)
    }

    /// Clips to a circle around `center` whose radius is `amount` of the
    /// distance to the farthest corner: 0 hides everything, 1 shows it all.
    #[must_use]
    pub fn reveal_circle(mut self, center: Alignment, amount: impl Into<PoseValue>) -> Self {
        self.reveal_center = Some(center);
        self.set(Prop::RevealCircle, amount)
    }

    /// The transition used when *entering* this pose. Leaving it uses the
    /// destination's transition.
    #[must_use]
    pub fn transition(mut self, transition: impl Into<Transition>) -> Self {
        self.transition = Some(transition.into());
        self
    }
}
