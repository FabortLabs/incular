//! Box decoration primitives and their painting data.

use incular_config::TextDirection;
use incular_core::{Color, Lerp, Offset, Rect};
use incular_image::ImageHandle;
use incular_rendering::{
    BlendMode, Brush, FilterQuality, GradientStops, LinearGradient, Paint, RadialGradient,
    SweepGradient,
};
use smart_default::SmartDefault;
use typed_builder::TypedBuilder;

use super::geometry::{BorderRadius, BorderRadiusDirectional, BorderRadiusGeometry};
use crate::{
    AlignmentGeometry,
    tree::{ImageFit, ImageRepeat},
};
use std::{
    hash::{Hash, Hasher},
    sync::Arc,
};

/// The style of a border stroke.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum BorderStyle {
    None,
    #[default]
    Solid,
}

/// One side of a border.
#[derive(Clone, Copy, Debug, PartialEq, TypedBuilder, SmartDefault)]
pub struct BorderSide {
    #[builder(default = Color::BLACK)]
    #[default(Color::BLACK)]
    pub color: Color,
    #[builder(
        default = 1.0,
        setter(transform = |width: f32| checked_border_width(width))
    )]
    #[default(1.0)]
    pub width: f32,
    #[builder(default = BorderStyle::Solid)]
    pub style: BorderStyle,
    #[builder(default = -1.0)]
    #[default(-1.0)]
    pub stroke_align: f32,
}

impl Hash for BorderSide {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.color.hash(state);
        self.style.hash(state);
        super::hash_paint_float(self.width, state);
        super::hash_paint_float(self.stroke_align, state);
    }
}

const fn checked_border_width(width: f32) -> f32 {
    debug_assert!(
        width >= 0.0,
        "border side width must not be negative or NaN"
    );
    if width >= 0.0 { width } else { 0.0 }
}

impl BorderSide {
    pub const NONE: Self = Self {
        color: Color::BLACK,
        width: 0.0,
        style: BorderStyle::None,
        stroke_align: -1.0,
    };

    /// # Panics
    /// In debug builds, panics if the width is negative or NaN.
    #[must_use]
    pub const fn new(color: Color, width: f32, style: BorderStyle) -> Self {
        Self {
            color,
            width: checked_border_width(width),
            style,
            stroke_align: -1.0,
        }
    }

    #[must_use]
    pub const fn solid(color: Color, width: f32) -> Self {
        Self::new(color, width, BorderStyle::Solid)
    }

    #[must_use]
    pub const fn none() -> Self {
        Self::NONE
    }

    /// Copies this side, replacing only the supplied properties.
    ///
    /// # Panics
    ///
    /// In debug builds, panics if the resulting width is negative or NaN.
    #[must_use]
    pub fn copy_with(
        self,
        color: Option<Color>,
        width: Option<f32>,
        style: Option<BorderStyle>,
        stroke_align: Option<f32>,
    ) -> Self {
        let width = width.unwrap_or(self.width);
        debug_assert!(
            width >= 0.0,
            "border side width must not be negative or NaN"
        );
        Self {
            color: color.unwrap_or(self.color),
            width,
            style: style.unwrap_or(self.style),
            stroke_align: stroke_align.unwrap_or(self.stroke_align),
        }
    }

    /// The stroke paint for this side. Alignment affects the path, not paint.
    #[must_use]
    pub fn to_paint(self) -> Paint {
        let (color, width) = match self.style {
            BorderStyle::Solid => (self.color, self.width),
            BorderStyle::None => (Color::TRANSPARENT, 0.0),
        };
        Paint::new().color(color).stroke_width(width).stroke()
    }

    /// Zero-width absent sides merge with anything. Other sides must agree
    /// in style and color; their widths and alignments may differ.
    #[must_use]
    pub fn can_merge(&self, other: &Self) -> bool {
        self.is_absent()
            || other.is_absent()
            || (self.style == other.style && self.color == other.color)
    }

    /// Adds two compatible sides, preserving the farthest outside alignment.
    ///
    /// # Panics
    ///
    /// Panics when [`Self::can_merge`] returns false.
    #[must_use]
    pub fn merge(&self, other: &Self) -> Self {
        assert!(self.can_merge(other), "incompatible border sides");
        match (self.is_absent(), other.is_absent()) {
            (true, true) => Self::NONE,
            (true, false) => *other,
            (false, true) => *self,
            (false, false) => Self {
                width: self.width + other.width,
                stroke_align: self.stroke_align.max(other.stroke_align),
                ..*self
            },
        }
    }

    fn is_absent(&self) -> bool {
        self.style == BorderStyle::None && self.width == 0.0
    }

    /// Flutter `strokeInset`: how far the stroke reaches inside the shape.
    #[must_use]
    pub const fn stroke_inset(&self) -> f32 {
        self.width * (1.0 - (1.0 + self.stroke_align) / 2.0)
    }

    /// Flutter `strokeOutset`: how far the stroke reaches outside the shape.
    #[must_use]
    pub fn stroke_outset(&self) -> f32 {
        self.width * (1.0 + self.stroke_align) / 2.0
    }

    /// Flutter `strokeOffset`: the stroke center's distance outside the
    /// shape's edge, doubled.
    #[must_use]
    pub fn stroke_offset(&self) -> f32 {
        self.width * self.stroke_align
    }

    /// Scales the width, resets alignment to inside, and removes the stroke
    /// for non-positive factors instead of producing a solid hairline.
    #[must_use]
    pub fn scale(&self, t: f32) -> Self {
        Self {
            width: (self.width * t).max(0.0),
            style: if t <= 0.0 {
                BorderStyle::None
            } else {
                self.style
            },
            stroke_align: Self::NONE.stroke_align,
            color: self.color,
        }
    }
}

/// Flutter `BorderSide.lerp`: negative extrapolation removes the side,
/// and transitions involving different styles fade through transparent
/// strokes without abruptly swapping the style at the midpoint.
impl incular_core::Lerp for BorderSide {
    fn lerp(&self, other: &Self, t: f32) -> Self {
        if t == 0.0 || self == other {
            return *self;
        }
        if t == 1.0 {
            return *other;
        }
        let width = if self.width == other.width {
            self.width
        } else {
            self.width.lerp(&other.width, t)
        };
        if width < 0.0 {
            return Self::NONE;
        }
        if self.style == other.style && self.stroke_align == other.stroke_align {
            return Self {
                color: self.color.lerp(&other.color, t),
                width,
                ..*self
            };
        }
        let visible_color = |side: &Self| match side.style {
            BorderStyle::None => Color {
                alpha: 0,
                ..side.color
            },
            BorderStyle::Solid => side.color,
        };
        Self {
            color: visible_color(self).lerp(&visible_color(other), t),
            width,
            style: BorderStyle::Solid,
            stroke_align: if self.stroke_align == other.stroke_align {
                self.stroke_align
            } else {
                self.stroke_align.lerp(&other.stroke_align, t)
            },
        }
    }
}

/// A box border configuration specifying stroke properties on all four sides.
///
/// All sides default to [`BorderSide::NONE`].
#[derive(Clone, Copy, Debug, PartialEq, TypedBuilder)]
pub struct Border {
    #[builder(default = BorderSide::NONE)]
    pub top: BorderSide,
    #[builder(default = BorderSide::NONE)]
    pub right: BorderSide,
    #[builder(default = BorderSide::NONE)]
    pub bottom: BorderSide,
    #[builder(default = BorderSide::NONE)]
    pub left: BorderSide,
}

/// Alias for [`Border`], matching Flutter naming.
pub type BoxBorder = Border;

impl Default for Border {
    fn default() -> Self {
        Self::all(BorderSide::NONE)
    }
}

impl Border {
    #[must_use]
    pub fn new(width: f32, color: incular_core::Color) -> Self {
        Self::all(BorderSide::solid(color, width))
    }

    #[must_use]
    pub const fn all(side: BorderSide) -> Self {
        Self {
            top: side,
            right: side,
            bottom: side,
            left: side,
        }
    }

    /// Sets the left/right (vertical) sides and top/bottom (horizontal) sides.
    #[must_use]
    pub const fn symmetric(vertical: BorderSide, horizontal: BorderSide) -> Self {
        Self {
            top: horizontal,
            right: vertical,
            bottom: horizontal,
            left: vertical,
        }
    }

    #[must_use]
    pub const fn only(
        top: BorderSide,
        right: BorderSide,
        bottom: BorderSide,
        left: BorderSide,
    ) -> Self {
        Self {
            top,
            right,
            bottom,
            left,
        }
    }

    /// Flutter `BoxBorder.fromLTRB`, in physical edge order.
    #[must_use]
    pub const fn from_ltrb(
        left: BorderSide,
        top: BorderSide,
        right: BorderSide,
        bottom: BorderSide,
    ) -> Self {
        Self::only(top, right, bottom, left)
    }

    /// Flutter `BoxBorder.fromSTEB`, preserving logical start and end
    /// until the border is resolved against a text direction.
    #[must_use]
    pub const fn from_steb(
        start: BorderSide,
        top: BorderSide,
        end: BorderSide,
        bottom: BorderSide,
    ) -> BorderDirectional {
        BorderDirectional::only(top, start, end, bottom)
    }

    #[must_use]
    pub const fn from_border_side(side: BorderSide) -> Self {
        Self::all(side)
    }

    /// Combines corresponding sides without resolving incompatible strokes.
    ///
    /// # Panics
    ///
    /// Panics if any pair of sides cannot be merged.
    #[must_use]
    pub fn merge(self, other: Self) -> Self {
        Self::only(
            self.top.merge(&other.top),
            self.right.merge(&other.right),
            self.bottom.merge(&other.bottom),
            self.left.merge(&other.left),
        )
    }

    /// Flutter `Border.dimensions`: each side's `strokeInset`, the padding
    /// a decoration with this border adds.
    #[must_use]
    pub const fn dimensions(self) -> incular_config::EdgeInsets {
        incular_config::EdgeInsets::only(
            self.left.stroke_inset(),
            self.top.stroke_inset(),
            self.right.stroke_inset(),
            self.bottom.stroke_inset(),
        )
    }

    /// Alias for [`Self::dimensions`].
    #[must_use]
    pub const fn stroke_insets(self) -> incular_config::EdgeInsets {
        self.dimensions()
    }

    /// Flutter `Border.scale`: every side scaled by `t`.
    #[must_use]
    pub fn scale(self, t: f32) -> Self {
        Self {
            top: self.top.scale(t),
            right: self.right.scale(t),
            bottom: self.bottom.scale(t),
            left: self.left.scale(t),
        }
    }

    /// Interpolates borders, scaling the present endpoint when the other is absent.
    #[must_use]
    pub fn lerp_optional(begin: Option<Self>, end: Option<Self>, t: f32) -> Option<Self> {
        match (begin, end) {
            (Some(a), Some(b)) => Some(a.lerp(&b, t)),
            (Some(a), None) => Some(a.scale(1.0 - t)),
            (None, Some(b)) => Some(b.scale(t)),
            (None, None) => None,
        }
    }

    /// Flutter `Border.isUniform`: every side has the same color, width,
    /// style and stroke alignment.
    #[must_use]
    pub fn is_uniform(&self) -> bool {
        [self.right, self.bottom, self.left].iter().all(|side| {
            side.color == self.top.color
                && side.width == self.top.width
                && side.style == self.top.style
                && side.stroke_align == self.top.stroke_align
        })
    }
}

impl incular_core::Lerp for Border {
    fn lerp(&self, other: &Self, t: f32) -> Self {
        Self {
            top: self.top.lerp(&other.top, t),
            right: self.right.lerp(&other.right, t),
            bottom: self.bottom.lerp(&other.bottom, t),
            left: self.left.lerp(&other.left, t),
        }
    }
}

impl From<Border> for incular_rendering::Border {
    fn from(b: Border) -> Self {
        incular_rendering::Border::new(b.top.width, b.top.color)
    }
}

/// Directional border with start and end sides. Omitted sides do not paint.
#[derive(Clone, Copy, Debug, PartialEq, TypedBuilder, SmartDefault)]
pub struct BorderDirectional {
    #[builder(default = BorderSide::NONE)]
    #[default(BorderSide::NONE)]
    pub top: BorderSide,
    #[builder(default = BorderSide::NONE)]
    #[default(BorderSide::NONE)]
    pub start: BorderSide,
    #[builder(default = BorderSide::NONE)]
    #[default(BorderSide::NONE)]
    pub end: BorderSide,
    #[builder(default = BorderSide::NONE)]
    #[default(BorderSide::NONE)]
    pub bottom: BorderSide,
}

impl BorderDirectional {
    #[must_use]
    pub const fn only(
        top: BorderSide,
        start: BorderSide,
        end: BorderSide,
        bottom: BorderSide,
    ) -> Self {
        Self {
            top,
            start,
            end,
            bottom,
        }
    }

    /// The part of each stroke inside the painted bounds, in logical edge order.
    #[must_use]
    pub const fn dimensions(self) -> incular_config::EdgeInsetsDirectional {
        incular_config::EdgeInsetsDirectional::only(
            self.start.stroke_inset(),
            self.top.stroke_inset(),
            self.end.stroke_inset(),
            self.bottom.stroke_inset(),
        )
    }

    /// Combines corresponding sides without collapsing start and end.
    ///
    /// # Panics
    ///
    /// Panics if any pair of sides cannot be merged.
    #[must_use]
    pub fn merge(self, other: Self) -> Self {
        Self::only(
            self.top.merge(&other.top),
            self.start.merge(&other.start),
            self.end.merge(&other.end),
            self.bottom.merge(&other.bottom),
        )
    }

    /// Scales each side independently, preserving logical edge order.
    #[must_use]
    pub fn scale(self, t: f32) -> Self {
        Self::only(
            self.top.scale(t),
            self.start.scale(t),
            self.end.scale(t),
            self.bottom.scale(t),
        )
    }

    /// Interpolates logical sides without resolving either border's direction.
    #[must_use]
    pub fn lerp_optional(begin: Option<Self>, end: Option<Self>, t: f32) -> Option<Self> {
        match (begin, end) {
            (Some(a), Some(b)) => Some(a.lerp(&b, t)),
            (Some(a), None) => Some(a.scale(1.0 - t)),
            (None, Some(b)) => Some(b.scale(t)),
            (None, None) => None,
        }
    }

    /// Whether color, width, style and stroke alignment agree on every side.
    #[must_use]
    pub fn is_uniform(&self) -> bool {
        self.resolve(incular_config::TextDirection::Ltr)
            .is_uniform()
    }

    #[must_use]
    pub const fn resolve(self, direction: incular_config::TextDirection) -> Border {
        match direction {
            incular_config::TextDirection::Ltr => Border {
                top: self.top,
                right: self.end,
                bottom: self.bottom,
                left: self.start,
            },
            incular_config::TextDirection::Rtl => Border {
                top: self.top,
                right: self.start,
                bottom: self.bottom,
                left: self.end,
            },
        }
    }
}

impl incular_core::Lerp for BorderDirectional {
    fn lerp(&self, other: &Self, t: f32) -> Self {
        Self {
            top: self.top.lerp(&other.top, t),
            start: self.start.lerp(&other.start, t),
            end: self.end.lerp(&other.end, t),
            bottom: self.bottom.lerp(&other.bottom, t),
        }
    }
}

/// The geometric shape of a box.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum BoxShape {
    #[default]
    Rectangle,
    Circle,
}

/// Blur style for shadows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum BlurStyle {
    #[default]
    Normal,
    Solid,
    Outer,
    Inner,
}

/// A shadow cast by a box.
///
/// By default the shadow is solid black with zero offset, zero blur radius,
/// zero spread radius, and [`BlurStyle::Normal`].
#[derive(Clone, Copy, Debug, PartialEq, TypedBuilder)]
pub struct BoxShadow {
    #[builder(default = Color::BLACK)]
    pub color: Color,
    #[builder(default = Offset::default())]
    pub offset: incular_core::Offset,
    #[builder(default = 0.0)]
    pub blur_radius: f32,
    #[builder(default = 0.0)]
    pub spread_radius: f32,
    /// Flutter `blurStyle`: how the blurred shadow combines with its shape
    /// (Skia blur mask styles).
    #[builder(default = BlurStyle::Normal)]
    pub blur_style: BlurStyle,
}

impl BoxShadow {
    #[must_use]
    pub const fn new(
        color: Color,
        offset: incular_core::Offset,
        blur_radius: f32,
        spread_radius: f32,
    ) -> Self {
        Self {
            color,
            offset,
            blur_radius,
            spread_radius,
            blur_style: BlurStyle::Normal,
        }
    }
}

impl Default for BoxShadow {
    fn default() -> Self {
        Self::new(Color::BLACK, Offset::default(), 0.0, 0.0)
    }
}

impl BoxShadow {
    /// Flutter `BoxShadow.scale`: the geometry scales, the color does not.
    #[must_use]
    pub fn scale(&self, factor: f32) -> Self {
        Self {
            offset: Offset::new(self.offset.x * factor, self.offset.y * factor),
            blur_radius: self.blur_radius * factor,
            spread_radius: self.spread_radius * factor,
            ..*self
        }
    }

    /// Flutter `BoxShadow.lerpList`: shadows pair up by index, and the
    /// extra ones of the longer list scale out or in.
    #[must_use]
    pub fn lerp_list(a: &[Self], b: &[Self], t: f32) -> Vec<Self> {
        let common = a.len().min(b.len());
        a.iter()
            .zip(b)
            .map(|(a, b)| incular_core::Lerp::lerp(a, b, t))
            .chain(a[common..].iter().map(|shadow| shadow.scale(1.0 - t)))
            .chain(b[common..].iter().map(|shadow| shadow.scale(t)))
            .collect()
    }
}

impl incular_core::Lerp for BoxShadow {
    /// Flutter `BoxShadow.lerp`. The blur style is the first non-normal
    /// one.
    fn lerp(&self, other: &Self, t: f32) -> Self {
        Self {
            color: self.color.lerp_unclamped(other.color, t),
            offset: self.offset.lerp(&other.offset, t),
            blur_radius: self.blur_radius.lerp(&other.blur_radius, t),
            spread_radius: self.spread_radius.lerp(&other.spread_radius, t),
            blur_style: if self.blur_style == BlurStyle::Normal {
                other.blur_style
            } else {
                self.blur_style
            },
        }
    }
}

pub use incular_rendering::TileMode;

/// An image configuration drawn inside a box decoration.
#[derive(Clone, Debug, PartialEq, TypedBuilder)]
pub struct DecorationImage {
    pub image: ImageHandle,
    /// Flutter `fit`. `None` resolves like Flutter's `paintImage`: scale-down
    /// without a center slice, fill with one. See [`Self::resolved_fit`].
    #[builder(default, setter(strip_option))]
    pub fit: Option<ImageFit>,
    #[builder(default = incular_config::Alignment::CENTER)]
    pub alignment: incular_config::Alignment,
    /// Optional logical or mixed alignment, resolved by the decoration painter.
    /// When set, this takes precedence over the physical `alignment` field.
    #[builder(default, setter(strip_option, into))]
    pub alignment_geometry: Option<AlignmentGeometry>,
    /// Flutter `centerSlice`, in logical image pixels: the image paints as
    /// a nine-slice (`Canvas.drawImageNine`), and a missing fit is fill.
    #[builder(default, setter(strip_option))]
    pub center_slice: Option<incular_core::Rect>,
    #[builder(default = ImageRepeat::NoRepeat)]
    pub repeat: ImageRepeat,
    /// Flutter `matchTextDirection`: the image mirrors horizontally under a
    /// right-to-left ambient `Directionality`.
    #[builder(default = false)]
    pub match_text_direction: bool,
    #[builder(default = 1.0)]
    pub scale: f32,
    #[builder(default = 1.0)]
    pub opacity: f32,
    /// Flutter `filterQuality`, default [`FilterQuality::Medium`].
    #[builder(default = FilterQuality::Medium)]
    pub filter_quality: FilterQuality,
}

impl DecorationImage {
    /// Flutter `paintImage`'s null-fit rule: `BoxFit.scaleDown` when there is
    /// no center slice, `BoxFit.fill` when there is one.
    #[must_use]
    pub fn resolved_fit(&self) -> ImageFit {
        self.fit.unwrap_or(if self.center_slice.is_none() {
            ImageFit::ScaleDown
        } else {
            ImageFit::Fill
        })
    }

    /// Sets alignment in physical or logical coordinates.
    #[must_use]
    pub fn alignment_geometry(mut self, alignment: impl Into<AlignmentGeometry>) -> Self {
        self.alignment_geometry = Some(alignment.into());
        self
    }

    #[must_use]
    pub fn new(image: ImageHandle) -> Self {
        Self {
            image,
            fit: None,
            alignment: incular_config::Alignment::CENTER,
            alignment_geometry: None,
            center_slice: None,
            repeat: ImageRepeat::NoRepeat,
            match_text_direction: false,
            scale: 1.0,
            opacity: 1.0,
            filter_quality: FilterQuality::Medium,
        }
    }
}

/// Linear gradient with endpoints relative to the painted box. Unlike a
/// renderer brush in logical pixels, an endpoint can follow text direction.
#[derive(Clone, Debug, PartialEq)]
pub struct BoxLinearGradient {
    pub begin: AlignmentGeometry,
    pub end: AlignmentGeometry,
    pub stops: GradientStops,
    pub tile_mode: TileMode,
    /// A clockwise shader rotation about the paint bounds.
    pub rotation: Option<f32>,
}

impl BoxLinearGradient {
    #[must_use]
    pub fn new(
        begin: impl Into<AlignmentGeometry>,
        end: impl Into<AlignmentGeometry>,
        stops: GradientStops,
    ) -> Self {
        Self {
            begin: begin.into(),
            end: end.into(),
            stops,
            tile_mode: TileMode::Clamp,
            rotation: None,
        }
    }

    /// Scales opacity, leaving geometry, colors' RGB and tile mode intact.
    #[must_use]
    pub fn scale(&self, factor: f32) -> Self {
        Self {
            stops: self.stops.scale_opacity(factor),
            ..self.clone()
        }
    }

    /// Uses a single color at every existing stop.
    #[must_use]
    pub fn from_color(&self, color: Color) -> Self {
        Self {
            stops: self.stops.with_color(color),
            ..self.clone()
        }
    }

    /// Replaces every color's opacity without scaling the old alpha.
    #[must_use]
    pub fn with_opacity(&self, opacity: f32) -> Self {
        Self {
            stops: self.stops.with_opacity(opacity),
            ..self.clone()
        }
    }

    /// Sets a shader rotation in radians.
    #[must_use]
    pub fn rotated(mut self, radians: f32) -> Self {
        self.rotation = Some(radians);
        self
    }

    /// Interpolates two gradients, fading from or to transparent when absent.
    #[must_use]
    pub fn lerp_optional(a: Option<&Self>, b: Option<&Self>, t: f32) -> Option<Self> {
        match (a, b) {
            (None, None) => None,
            (Some(a), None) => Some(a.scale(1. - t)),
            (None, Some(b)) => Some(b.scale(t)),
            (Some(a), Some(b)) => Some(a.lerp(b, t)),
        }
    }

    /// Creates a shader with an optional ambient direction. Logical
    /// alignments require that direction; physical alignments do not.
    pub fn create_shader(
        &self,
        rect: Rect,
        direction: Option<TextDirection>,
    ) -> Result<LinearGradient, crate::tree::TreeError> {
        let needs_direction = !matches!(self.begin, AlignmentGeometry::Physical(_))
            || !matches!(self.end, AlignmentGeometry::Physical(_));
        if needs_direction && direction.is_none() {
            return Err(crate::tree::TreeError::MissingDirectionality {
                widget: "BoxLinearGradient",
                reason: "resolving logical gradient alignment",
            });
        }
        Ok(self.resolve(rect, direction.unwrap_or(TextDirection::Ltr)))
    }

    /// Resolves the shader in the coordinate space of the supplied box.
    #[must_use]
    pub fn resolve(&self, rect: Rect, direction: TextDirection) -> LinearGradient {
        let endpoint = |alignment: AlignmentGeometry| {
            let alignment = alignment.resolve(direction);
            Offset::new(
                rect.origin.x + (alignment.x + 1.0) * 0.5 * rect.size.width,
                rect.origin.y + (alignment.y + 1.0) * 0.5 * rect.size.height,
            )
        };
        let gradient = LinearGradient {
            start: endpoint(self.begin),
            end: endpoint(self.end),
            stops: self.stops.clone(),
            tile_mode: self.tile_mode,
        };
        if let Some(rotation) = self.rotation {
            let Brush::LinearGradient(rotated) =
                Brush::LinearGradient(gradient).rotated(rotation, rect)
            else {
                unreachable!("rotation preserves the gradient kind")
            };
            rotated
        } else {
            gradient
        }
    }
}

impl Lerp for BoxLinearGradient {
    fn lerp(&self, other: &Self, t: f32) -> Self {
        if self == other {
            return self.clone();
        }
        Self {
            begin: self.begin.lerp(&other.begin, t),
            end: self.end.lerp(&other.end, t),
            stops: self.stops.interpolate(&other.stops, t),
            rotation: if t < 0.5 {
                self.rotation
            } else {
                other.rotation
            },
            tile_mode: if t < 0.5 {
                self.tile_mode
            } else {
                other.tile_mode
            },
        }
    }
}

/// Radial gradient with the center and focal point relative to its box.
/// Both radii are fractions of that box's shortest side.
#[derive(Clone, Debug, PartialEq)]
pub struct BoxRadialGradient {
    pub center: AlignmentGeometry,
    pub radius: f32,
    pub focal: Option<AlignmentGeometry>,
    pub focal_radius: f32,
    pub stops: GradientStops,
    pub tile_mode: TileMode,
    pub rotation: Option<f32>,
}

impl BoxRadialGradient {
    #[must_use]
    pub fn new(stops: GradientStops) -> Self {
        Self {
            center: incular_config::Alignment::CENTER.into(),
            radius: 0.5,
            focal: None,
            focal_radius: 0.,
            stops,
            tile_mode: TileMode::Clamp,
            rotation: None,
        }
    }

    #[must_use]
    pub fn center(mut self, value: impl Into<AlignmentGeometry>) -> Self {
        self.center = value.into();
        self
    }

    #[must_use]
    pub fn radius(mut self, value: f32) -> Self {
        self.radius = value.max(0.);
        self
    }

    #[must_use]
    pub fn focal(mut self, value: impl Into<AlignmentGeometry>, radius: f32) -> Self {
        self.focal = Some(value.into());
        self.focal_radius = radius.max(0.);
        self
    }

    #[must_use]
    pub fn rotated(mut self, radians: f32) -> Self {
        self.rotation = Some(radians);
        self
    }

    #[must_use]
    pub fn scale(&self, factor: f32) -> Self {
        Self {
            stops: self.stops.scale_opacity(factor),
            ..self.clone()
        }
    }

    #[must_use]
    pub fn from_color(&self, color: Color) -> Self {
        Self {
            stops: self.stops.with_color(color),
            ..self.clone()
        }
    }

    #[must_use]
    pub fn with_opacity(&self, opacity: f32) -> Self {
        Self {
            stops: self.stops.with_opacity(opacity),
            ..self.clone()
        }
    }

    #[must_use]
    pub fn lerp_optional(a: Option<&Self>, b: Option<&Self>, t: f32) -> Option<Self> {
        match (a, b) {
            (None, None) => None,
            (Some(a), None) => Some(a.scale(1. - t)),
            (None, Some(b)) => Some(b.scale(t)),
            (Some(a), Some(b)) => Some(a.lerp(b, t)),
        }
    }

    /// Creates a shader with an optional ambient direction. Logical
    /// alignments require that direction; physical alignments do not.
    pub fn create_shader(
        &self,
        rect: Rect,
        direction: Option<TextDirection>,
    ) -> Result<RadialGradient, crate::tree::TreeError> {
        let needs_direction = !matches!(self.center, AlignmentGeometry::Physical(_))
            || self
                .focal
                .is_some_and(|focal| !matches!(focal, AlignmentGeometry::Physical(_)));
        if needs_direction && direction.is_none() {
            return Err(crate::tree::TreeError::MissingDirectionality {
                widget: "BoxRadialGradient",
                reason: "resolving logical gradient alignment",
            });
        }
        Ok(self.resolve(rect, direction.unwrap_or(TextDirection::Ltr)))
    }

    #[must_use]
    pub fn resolve(&self, rect: Rect, direction: TextDirection) -> RadialGradient {
        let point = |alignment: AlignmentGeometry| {
            let alignment = alignment.resolve(direction);
            Offset::new(
                rect.origin.x + (alignment.x + 1.) * 0.5 * rect.size.width,
                rect.origin.y + (alignment.y + 1.) * 0.5 * rect.size.height,
            )
        };
        let shortest = rect.size.width.min(rect.size.height);
        let gradient = RadialGradient {
            center: point(self.center),
            radius: self.radius * shortest,
            focal: self.focal.map(point),
            focal_radius: self.focal_radius * shortest,
            stops: self.stops.clone(),
            tile_mode: self.tile_mode,
        };
        if let Some(rotation) = self.rotation {
            let Brush::RadialGradient(rotated) =
                Brush::RadialGradient(gradient).rotated(rotation, rect)
            else {
                unreachable!("rotation preserves the gradient kind");
            };
            rotated
        } else {
            gradient
        }
    }
}

impl Lerp for BoxRadialGradient {
    fn lerp(&self, other: &Self, t: f32) -> Self {
        if self == other {
            return self.clone();
        }
        let center: AlignmentGeometry = incular_config::Alignment::CENTER.into();
        let mix_radius = |a, b| f32::max(a + (b - a) * t, 0.);
        Self {
            center: self.center.lerp(&other.center, t),
            radius: mix_radius(self.radius, other.radius),
            focal: match (self.focal, other.focal) {
                (None, None) => None,
                (a, b) => Some(a.unwrap_or(center).lerp(&b.unwrap_or(center), t)),
            },
            focal_radius: mix_radius(self.focal_radius, other.focal_radius),
            stops: self.stops.interpolate(&other.stops, t),
            tile_mode: if t < 0.5 {
                self.tile_mode
            } else {
                other.tile_mode
            },
            rotation: if t < 0.5 {
                self.rotation
            } else {
                other.rotation
            },
        }
    }
}

/// Angular gradient with a box-relative or directional center.
#[derive(Clone, Debug, PartialEq)]
pub struct BoxSweepGradient {
    pub center: AlignmentGeometry,
    pub start_angle: f32,
    pub end_angle: f32,
    pub stops: GradientStops,
    pub tile_mode: TileMode,
    pub rotation: Option<f32>,
}

impl BoxSweepGradient {
    #[must_use]
    pub fn new(stops: GradientStops) -> Self {
        Self {
            center: incular_config::Alignment::CENTER.into(),
            start_angle: 0.,
            end_angle: std::f32::consts::TAU,
            stops,
            tile_mode: TileMode::Clamp,
            rotation: None,
        }
    }

    #[must_use]
    pub fn center(mut self, value: impl Into<AlignmentGeometry>) -> Self {
        self.center = value.into();
        self
    }

    #[must_use]
    pub fn angles(mut self, start: f32, end: f32) -> Self {
        self.start_angle = start;
        self.end_angle = end;
        self
    }

    #[must_use]
    pub fn rotated(mut self, radians: f32) -> Self {
        self.rotation = Some(radians);
        self
    }

    #[must_use]
    pub fn scale(&self, factor: f32) -> Self {
        Self {
            stops: self.stops.scale_opacity(factor),
            ..self.clone()
        }
    }

    #[must_use]
    pub fn from_color(&self, color: Color) -> Self {
        Self {
            stops: self.stops.with_color(color),
            ..self.clone()
        }
    }

    #[must_use]
    pub fn with_opacity(&self, opacity: f32) -> Self {
        Self {
            stops: self.stops.with_opacity(opacity),
            ..self.clone()
        }
    }

    #[must_use]
    pub fn lerp_optional(a: Option<&Self>, b: Option<&Self>, t: f32) -> Option<Self> {
        match (a, b) {
            (None, None) => None,
            (Some(a), None) => Some(a.scale(1. - t)),
            (None, Some(b)) => Some(b.scale(t)),
            (Some(a), Some(b)) => Some(a.lerp(b, t)),
        }
    }

    /// Creates a shader with an optional ambient direction. Logical
    /// alignments require that direction; physical alignments do not.
    pub fn create_shader(
        &self,
        rect: Rect,
        direction: Option<TextDirection>,
    ) -> Result<SweepGradient, crate::tree::TreeError> {
        if !matches!(self.center, AlignmentGeometry::Physical(_)) && direction.is_none() {
            return Err(crate::tree::TreeError::MissingDirectionality {
                widget: "BoxSweepGradient",
                reason: "resolving logical gradient alignment",
            });
        }
        Ok(self.resolve(rect, direction.unwrap_or(TextDirection::Ltr)))
    }

    #[must_use]
    pub fn resolve(&self, rect: Rect, direction: TextDirection) -> SweepGradient {
        let center = self.center.resolve(direction);
        let center = Offset::new(
            rect.origin.x + (center.x + 1.) * 0.5 * rect.size.width,
            rect.origin.y + (center.y + 1.) * 0.5 * rect.size.height,
        );
        let mut gradient = SweepGradient::new(center, self.start_angle, self.stops.clone())
            .with_end_angle(self.end_angle);
        gradient.tile_mode = self.tile_mode;
        if let Some(rotation) = self.rotation {
            let Brush::SweepGradient(rotated) =
                Brush::SweepGradient(gradient).rotated(rotation, rect)
            else {
                unreachable!("rotation preserves the gradient kind");
            };
            rotated
        } else {
            gradient
        }
    }
}

impl Lerp for BoxSweepGradient {
    fn lerp(&self, other: &Self, t: f32) -> Self {
        if self == other {
            return self.clone();
        }
        let mix_angle = |a, b| f32::max(a + (b - a) * t, 0.);
        Self {
            center: self.center.lerp(&other.center, t),
            start_angle: mix_angle(self.start_angle, other.start_angle),
            end_angle: mix_angle(self.end_angle, other.end_angle),
            stops: self.stops.interpolate(&other.stops, t),
            tile_mode: if t < 0.5 {
                self.tile_mode
            } else {
                other.tile_mode
            },
            rotation: if t < 0.5 {
                self.rotation
            } else {
                other.rotation
            },
        }
    }
}

/// An immutable description of how to paint a box.
#[derive(Clone, Debug, Default, PartialEq, TypedBuilder)]
pub struct BoxDecoration {
    #[builder(default, setter(strip_option))]
    pub color: Option<Color>,
    #[builder(default, setter(strip_option))]
    pub image: Option<DecorationImage>,
    #[builder(default, setter(strip_option))]
    pub border: Option<Border>,
    #[builder(default, setter(strip_option))]
    pub border_radius: Option<BorderRadius>,
    /// Logical radii are resolved at paint and hit-test time. An interpolated
    /// decoration can carry both physical and logical components.
    #[builder(default, setter(transform = |radius: BorderRadiusDirectional| Some(Arc::new(radius))))]
    pub directional_border_radius: Option<Arc<BorderRadiusDirectional>>,
    #[builder(
        default = Vec::new(),
        setter(transform = |shadows: impl IntoIterator<Item = BoxShadow>| {
            shadows.into_iter().collect::<Vec<BoxShadow>>()
        })
    )]
    pub box_shadow: Vec<BoxShadow>,
    #[builder(default, setter(strip_option, into))]
    pub gradient: Option<Brush>,
    /// An aligned shader, resolved against this decoration's rectangle.
    #[builder(default, setter(transform = |gradient: BoxLinearGradient| Some(Arc::new(gradient))))]
    pub aligned_gradient: Option<Arc<BoxLinearGradient>>,
    #[builder(default, setter(transform = |gradient: BoxRadialGradient| Some(Arc::new(gradient))))]
    pub aligned_radial_gradient: Option<Arc<BoxRadialGradient>>,
    #[builder(default, setter(transform = |gradient: BoxSweepGradient| Some(Arc::new(gradient))))]
    pub aligned_sweep_gradient: Option<Arc<BoxSweepGradient>>,
    /// Flutter `backgroundBlendMode`: the background color or gradient
    /// blends with what is painted below it (it needs a fill).
    #[builder(default, setter(strip_option))]
    pub background_blend_mode: Option<BlendMode>,
    #[builder(default = BoxShape::Rectangle)]
    pub shape: BoxShape,
}

impl BoxDecoration {
    pub(crate) const DIRECTIONAL_RADIUS_NEEDS_DIRECTION: &'static str = "No TextDirection found. BorderRadiusDirectional cannot resolve without a Directionality ancestor";

    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    #[must_use]
    pub fn image(mut self, image: DecorationImage) -> Self {
        self.image = Some(image);
        self
    }

    #[must_use]
    pub fn border(mut self, border: Border) -> Self {
        self.border = Some(border);
        self
    }

    #[must_use]
    pub fn border_radius(mut self, radius: BorderRadius) -> Self {
        self.border_radius = Some(radius);
        self.directional_border_radius = None;
        self
    }

    /// Sets corner radii measured from the leading and trailing edges.
    #[must_use]
    pub fn directional_border_radius(mut self, radius: BorderRadiusDirectional) -> Self {
        self.border_radius = None;
        self.directional_border_radius = Some(Arc::new(radius));
        self
    }

    /// Sets physical, directional, or mixed corner radii. Mixed values
    /// resolve against the text direction at paint and hit-test time.
    #[must_use]
    pub fn border_radius_geometry(mut self, radius: impl Into<BorderRadiusGeometry>) -> Self {
        match radius.into() {
            BorderRadiusGeometry::Physical(value) => self.border_radius(value),
            BorderRadiusGeometry::Directional(value) => self.directional_border_radius(value),
            BorderRadiusGeometry::Mixed {
                physical,
                directional,
            } => {
                self.border_radius = Some(physical);
                self.directional_border_radius = Some(Arc::new(directional));
                self
            }
        }
    }

    /// Resolves this decoration's logical corner radii for explicit painting
    /// or hit testing. Widgets resolve them from their nearest Directionality.
    #[must_use]
    pub fn resolved_for_direction(mut self, direction: TextDirection) -> Self {
        if self.directional_border_radius.is_some() {
            self.border_radius = self.effective_radius(direction);
            self.directional_border_radius = None;
        }
        self
    }

    pub(super) fn effective_radius(&self, direction: TextDirection) -> Option<BorderRadius> {
        let Some(logical) = &self.directional_border_radius else {
            return self.border_radius;
        };
        let logical = logical.resolve(direction);
        let Some(physical) = self.border_radius else {
            return Some(logical);
        };
        Some(physical + logical)
    }

    #[must_use]
    pub fn box_shadow<I>(mut self, shadows: I) -> Self
    where
        I: IntoIterator<Item = BoxShadow>,
    {
        self.box_shadow = shadows.into_iter().collect();
        self
    }

    #[must_use]
    pub fn gradient(mut self, gradient: impl Into<Brush>) -> Self {
        self.gradient = Some(gradient.into());
        self.aligned_gradient = None;
        self.aligned_radial_gradient = None;
        self.aligned_sweep_gradient = None;
        self
    }

    /// Sets a gradient using box-relative or logical start/end alignments.
    #[must_use]
    pub fn aligned_gradient(mut self, gradient: BoxLinearGradient) -> Self {
        self.gradient = None;
        self.aligned_gradient = Some(Arc::new(gradient));
        self.aligned_radial_gradient = None;
        self.aligned_sweep_gradient = None;
        self
    }

    /// Sets a radial shader whose geometry follows the decoration bounds.
    #[must_use]
    pub fn aligned_radial_gradient(mut self, gradient: BoxRadialGradient) -> Self {
        self.gradient = None;
        self.aligned_gradient = None;
        self.aligned_radial_gradient = Some(Arc::new(gradient));
        self.aligned_sweep_gradient = None;
        self
    }

    /// Sets a sweep shader whose center follows the decoration bounds.
    #[must_use]
    pub fn aligned_sweep_gradient(mut self, gradient: BoxSweepGradient) -> Self {
        self.gradient = None;
        self.aligned_gradient = None;
        self.aligned_radial_gradient = None;
        self.aligned_sweep_gradient = Some(Arc::new(gradient));
        self
    }

    fn aligned_gradient_kind(&self) -> Option<AlignedGradientKind> {
        if self.aligned_gradient.is_some() {
            Some(AlignedGradientKind::Linear)
        } else if self.aligned_radial_gradient.is_some() {
            Some(AlignedGradientKind::Radial)
        } else if self.aligned_sweep_gradient.is_some() {
            Some(AlignedGradientKind::Sweep)
        } else {
            None
        }
    }

    pub(crate) fn has_aligned_gradient(&self) -> bool {
        self.aligned_gradient_kind().is_some()
    }

    pub(crate) fn resolve_aligned_gradient(
        &self,
        rect: Rect,
        direction: TextDirection,
    ) -> Option<Brush> {
        if let Some(gradient) = &self.aligned_gradient {
            return Some(Brush::LinearGradient(gradient.resolve(rect, direction)));
        }
        if let Some(gradient) = &self.aligned_radial_gradient {
            return Some(Brush::RadialGradient(gradient.resolve(rect, direction)));
        }
        self.aligned_sweep_gradient
            .as_ref()
            .map(|gradient| Brush::SweepGradient(gradient.resolve(rect, direction)))
    }

    #[must_use]
    pub fn background_blend_mode(mut self, mode: BlendMode) -> Self {
        self.background_blend_mode = Some(mode);
        self
    }

    #[must_use]
    pub fn shape(mut self, shape: BoxShape) -> Self {
        self.shape = shape;
        self
    }

    #[must_use]
    pub fn is_valid(&self) -> bool {
        if self.shape == BoxShape::Circle
            && (self.border_radius.is_some() || self.directional_border_radius.is_some())
        {
            return false;
        }
        if self.background_blend_mode.is_some()
            && self.color.is_none()
            && self.gradient.is_none()
            && !self.has_aligned_gradient()
        {
            return false;
        }
        true
    }

    #[must_use]
    pub fn change_impact(&self, other: &Self) -> incular_core::ChangeImpact {
        if self == other {
            return incular_core::ChangeImpact::None;
        }
        if self.border != other.border || self.shape != other.shape {
            incular_core::ChangeImpact::Layout
        } else {
            incular_core::ChangeImpact::Paint
        }
    }

    #[must_use]
    pub fn invalidation(&self, other: &Self) -> incular_core::Invalidation {
        if self == other {
            return incular_core::Invalidation::NONE;
        }
        if self.border != other.border || self.shape != other.shape {
            incular_core::Invalidation::LAYOUT | incular_core::Invalidation::PAINT
        } else {
            incular_core::Invalidation::PAINT
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum AlignedGradientKind {
    Linear,
    Radial,
    Sweep,
}

impl incular_core::Lerp for BoxDecoration {
    /// Flutter `BoxDecoration.lerp`: a part present on one side only
    /// scales in or out (`Color.lerp(null, c, t)`, `BoxBorder.lerp`,
    /// `BoxShadow.lerpList`). Raw brushes and images switch at the midpoint,
    /// and, as in Flutter, a blend mode is kept only at the ends or when
    /// interpolating a decoration with itself.
    fn lerp(&self, other: &Self, t: f32) -> Self {
        if std::ptr::eq(self, other) || t == 0.0 {
            return self.clone();
        }
        if t == 1.0 {
            return other.clone();
        }
        let a_kind = self.aligned_gradient_kind();
        let b_kind = other.aligned_gradient_kind();
        let cross_kind = a_kind.is_some() && b_kind.is_some() && a_kind != b_kind;
        let (a, b, gradient_t) = if cross_kind {
            // Flutter fades out the first type in the first half, then
            // fades in the second type in the second half.
            if t < 0.5 {
                (Some(self), None, 2. * t)
            } else {
                (None, Some(other), 2. * t - 1.)
            }
        } else {
            (Some(self), Some(other), t)
        };
        let (aligned_gradient, aligned_radial_gradient, aligned_sweep_gradient) =
            if self.gradient.is_some() || other.gradient.is_some() {
                let endpoint = if t < 0.5 { self } else { other };
                (
                    endpoint.aligned_gradient.clone(),
                    endpoint.aligned_radial_gradient.clone(),
                    endpoint.aligned_sweep_gradient.clone(),
                )
            } else {
                (
                    BoxLinearGradient::lerp_optional(
                        a.and_then(|value| value.aligned_gradient.as_deref()),
                        b.and_then(|value| value.aligned_gradient.as_deref()),
                        gradient_t,
                    )
                    .map(Arc::new),
                    BoxRadialGradient::lerp_optional(
                        a.and_then(|value| value.aligned_radial_gradient.as_deref()),
                        b.and_then(|value| value.aligned_radial_gradient.as_deref()),
                        gradient_t,
                    )
                    .map(Arc::new),
                    BoxSweepGradient::lerp_optional(
                        a.and_then(|value| value.aligned_sweep_gradient.as_deref()),
                        b.and_then(|value| value.aligned_sweep_gradient.as_deref()),
                        gradient_t,
                    )
                    .map(Arc::new),
                )
            };
        Self {
            color: lerp_color(self.color, other.color, t),
            image: if t < 0.5 {
                self.image.clone()
            } else {
                other.image.clone()
            },
            border: Border::lerp_optional(self.border, other.border, t),
            border_radius: BorderRadius::lerp_optional(self.border_radius, other.border_radius, t),
            directional_border_radius: BorderRadiusDirectional::lerp_optional(
                self.directional_border_radius.as_deref().copied(),
                other.directional_border_radius.as_deref().copied(),
                t,
            )
            .map(Arc::new),
            box_shadow: BoxShadow::lerp_list(&self.box_shadow, &other.box_shadow, t),
            gradient: if t < 0.5 {
                self.gradient.clone()
            } else {
                other.gradient.clone()
            },
            aligned_gradient,
            aligned_radial_gradient,
            aligned_sweep_gradient,
            background_blend_mode: None,
            shape: if t < 0.5 { self.shape } else { other.shape },
        }
    }
}

/// Flutter `Color.lerp` with nullable ends: a missing color is the other
/// one with its alpha scaled.
fn lerp_color(a: Option<Color>, b: Option<Color>, t: f32) -> Option<Color> {
    let scale_alpha = |color: Color, factor: f32| Color {
        alpha: (f32::from(color.alpha) * factor).round().clamp(0.0, 255.0) as u8,
        ..color
    };
    match (a, b) {
        (Some(a), Some(b)) => Some(incular_core::Lerp::lerp(&a, &b, t)),
        (Some(a), None) => Some(scale_alpha(a, 1.0 - t)),
        (None, Some(b)) => Some(scale_alpha(b, t)),
        (None, None) => None,
    }
}
