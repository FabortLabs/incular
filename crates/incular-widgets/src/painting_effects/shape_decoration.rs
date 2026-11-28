//! Flutter's `ShapeDecoration` (`painting/shape_decoration.dart`) and
//! `ClipPath.shape` (`ShapeBorderClipper`): a [`ShapeBorder`] used as a
//! decoration and as a clip.

use std::{borrow::Cow, rc::Rc, sync::Arc};

use incular_animation::OptionalColorTween;
use incular_config::{Clip, EdgeInsetsGeometry, TextDirection};
use incular_core::{Color, Lerp, Offset, Rect, Size};
use incular_rendering::{Brush, Canvas, CornerRadii, DisplayList, FillRule, GradientStops};

use super::box_painter::DecorationPosition;
use super::decoration::{
    BorderSide, BorderStyle, BoxDecoration, BoxLinearGradient, BoxRadialGradient, BoxShadow, BoxShape,
    BoxSweepGradient, DecorationImage, faded_brush, faded_decoration_image,
    lerp_raw_brush,
};
use super::geometry::BorderRadiusGeometry;
use super::shape_borders::{CircleBorder, RoundedRectangleBorder, ShapeBorder, lerp_shape_border};
use crate::{
    Builder, ClipPath, ClipShape, CustomClipper, CustomPaint, IgnorePointer, LayoutBuilder,
    Positioned, Stack, StackFit, Widget,
};

/// A gradient whose coordinates are relative to a shape's paint box.
#[derive(Clone, Debug, PartialEq)]
pub enum AlignedShapeGradient {
    Linear(Arc<BoxLinearGradient>),
    Radial(Arc<BoxRadialGradient>),
    Sweep(Arc<BoxSweepGradient>),
}

impl AlignedShapeGradient {
    fn needs_text_direction(&self) -> bool {
        use crate::AlignmentGeometry;
        let needs = |alignment| !matches!(alignment, AlignmentGeometry::Physical(_));
        match self {
            Self::Linear(g) => needs(g.begin) || needs(g.end),
            Self::Radial(g) => needs(g.center) || g.focal.is_some_and(needs),
            Self::Sweep(g) => needs(g.center),
        }
    }

    fn resolve(&self, rect: Rect, direction: TextDirection) -> Brush {
        match self {
            Self::Linear(g) => g.resolve(rect, direction).into(),
            Self::Radial(g) => g.resolve(rect, direction).into(),
            Self::Sweep(g) => g.resolve(rect, direction).into(),
        }
    }

    fn scale(&self, factor: f32) -> Self {
        match self {
            Self::Linear(g) => Self::Linear(Arc::new(g.scale(factor))),
            Self::Radial(g) => Self::Radial(Arc::new(g.scale(factor))),
            Self::Sweep(g) => Self::Sweep(Arc::new(g.scale(factor))),
        }
    }

    fn from_color(&self, color: Color) -> Self {
        match self {
            Self::Linear(g) => Self::Linear(Arc::new(g.from_color(color))),
            Self::Radial(g) => Self::Radial(Arc::new(g.from_color(color))),
            Self::Sweep(g) => Self::Sweep(Arc::new(g.from_color(color))),
        }
    }
}

impl Lerp for AlignedShapeGradient {
    fn lerp(&self, other: &Self, t: f32) -> Self {
        match (self, other) {
            (Self::Linear(a), Self::Linear(b)) if Arc::ptr_eq(a, b) => Self::Linear(a.clone()),
            (Self::Linear(a), Self::Linear(b)) => Self::Linear(Arc::new(a.lerp(b, t))),
            (Self::Radial(a), Self::Radial(b)) if Arc::ptr_eq(a, b) => Self::Radial(a.clone()),
            (Self::Radial(a), Self::Radial(b)) => Self::Radial(Arc::new(a.lerp(b, t))),
            (Self::Sweep(a), Self::Sweep(b)) if Arc::ptr_eq(a, b) => Self::Sweep(a.clone()),
            (Self::Sweep(a), Self::Sweep(b)) => Self::Sweep(Arc::new(a.lerp(b, t))),
            (a, _) if t < 0.5 => a.scale(1. - 2. * t),
            (_, b) => b.scale(2. * t - 1.),
        }
    }
}

/// Flutter `ShapeDecoration`: a shape's interior filled with a color or
/// gradient, an image clipped to the shape, its shadows, and the shape's own
/// border on top.
#[derive(Clone)]
pub struct ShapeDecoration {
    pub color: Option<Color>,
    /// A gradient fill is mutually exclusive with [`ShapeDecoration::color`].
    pub gradient: Option<Brush>,
    /// A box-relative shader, kept unresolved until the shape is painted.
    pub aligned_gradient: Option<AlignedShapeGradient>,
    pub shadows: Vec<BoxShadow>,
    /// Flutter `ShapeDecoration.image`: painted over the color or gradient,
    /// clipped to the shape's inner path, excluding the border's stroke.
    pub image: Option<DecorationImage>,
    pub shape: Rc<dyn ShapeBorder>,
}

impl ShapeDecoration {
    #[must_use]
    pub fn new(shape: impl ShapeBorder + 'static) -> Self {
        Self {
            color: None,
            gradient: None,
            aligned_gradient: None,
            shadows: Vec::new(),
            image: None,
            shape: Rc::new(shape),
        }
    }

    /// Converts Flutter's rectangular or circular BoxDecoration without
    /// discarding logical borders, corner radii, or box-relative gradients.
    ///
    /// As in Flutter, a rounded rectangle or circle needs a uniform border
    /// and a shape's solid color and gradient are mutually exclusive.
    #[must_use]
    pub fn from_box_decoration(source: &BoxDecoration) -> Self {
        let custom = source.custom_border.as_ref();
        let logical = source.directional_border.as_deref();
        let uniform = custom.map_or_else(|| logical.map_or_else(
            || source.border.is_none_or(|border| border.is_uniform()),
            |border| border.is_uniform(),
        ), |border| border.behavior().is_uniform());
        let side = custom
            .map(|border| border.behavior().top())
            .or_else(|| logical.map(|border| border.top))
            .or_else(|| source.border.map(|border| border.top))
            .unwrap_or(BorderSide::NONE);
        let radius = match (
            source.border_radius,
            source.directional_border_radius.as_deref(),
        ) {
            (Some(physical), Some(directional)) => Some(BorderRadiusGeometry::Mixed {
                physical,
                directional: *directional,
            }),
            (Some(radius), None) => Some(radius.into()),
            (None, Some(radius)) => Some((*radius).into()),
            (None, None) => None,
        };
        let shape: Rc<dyn ShapeBorder> = match (source.shape, radius) {
            (BoxShape::Circle, _) => {
                debug_assert!(uniform, "a circular shape needs a uniform border");
                Rc::new(CircleBorder::new(side))
            }
            (BoxShape::Rectangle, Some(radius)) => {
                debug_assert!(uniform, "a rounded shape needs a uniform border");
                Rc::new(RoundedRectangleBorder::new(side, radius))
            }
            (BoxShape::Rectangle, None) => {
                if let Some(border) = custom {
                    Rc::new(border.clone())
                } else if let Some(border) = logical {
                    Rc::new(*border)
                } else {
                    Rc::new(source.border.unwrap_or_default())
                }
            }
        };
        let aligned_gradient = source
            .aligned_gradient
            .clone()
            .map(AlignedShapeGradient::Linear)
            .or_else(|| {
                source
                    .aligned_radial_gradient
                    .clone()
                    .map(AlignedShapeGradient::Radial)
            })
            .or_else(|| {
                source
                    .aligned_sweep_gradient
                    .clone()
                    .map(AlignedShapeGradient::Sweep)
            });
        debug_assert!(
            source.color.is_none() || (source.gradient.is_none() && aligned_gradient.is_none()),
            "color and gradient are mutually exclusive"
        );
        Self {
            color: source.color,
            gradient: source.gradient.clone(),
            aligned_gradient,
            shadows: source.box_shadow.clone(),
            image: source.image.clone(),
            shape,
        }
    }

    #[must_use]
    pub fn color(mut self, color: Color) -> Self {
        debug_assert!(
            self.gradient.is_none() && self.aligned_gradient.is_none(),
            "color and gradient are mutually exclusive"
        );
        self.color = Some(color);
        self
    }

    #[must_use]
    pub fn gradient(mut self, gradient: impl Into<Brush>) -> Self {
        debug_assert!(
            self.color.is_none(),
            "color and gradient are mutually exclusive"
        );
        self.gradient = Some(gradient.into());
        self.aligned_gradient = None;
        self
    }

    /// Sets a gradient relative to the eventual paint box and direction.
    #[must_use]
    pub fn aligned_gradient(mut self, gradient: AlignedShapeGradient) -> Self {
        debug_assert!(
            self.color.is_none(),
            "color and gradient are mutually exclusive"
        );
        self.gradient = None;
        self.aligned_gradient = Some(gradient);
        self
    }

    #[must_use]
    pub fn image(mut self, image: DecorationImage) -> Self {
        self.image = Some(image);
        self
    }

    #[must_use]
    pub fn shadows(mut self, shadows: impl IntoIterator<Item = BoxShadow>) -> Self {
        self.shadows = shadows.into_iter().collect();
        self
    }

    /// Interpolates optional shape decorations. Identical objects and exact
    /// present endpoints are returned by reference.
    #[must_use]
    pub fn lerp_optional<'a>(
        a: Option<&'a Self>,
        b: Option<&'a Self>,
        t: f32,
    ) -> Option<Cow<'a, Self>> {
        match (a, b) {
            (None, None) => None,
            (Some(a), Some(b)) if std::ptr::eq(a, b) || t == 0. => Some(Cow::Borrowed(a)),
            (Some(_), Some(b)) if t == 1. => Some(Cow::Borrowed(b)),
            (Some(a), Some(b)) => Some(Cow::Owned(lerp_shape_decoration(a, b, t))),
            (Some(a), None) => Some(Cow::Owned(a.scaled(1. - t))),
            (None, Some(b)) => Some(Cow::Owned(b.scaled(t))),
        }
    }

    pub(crate) fn scaled(&self, factor: f32) -> Self {
        Self {
            color: OptionalColorTween::new(None, self.color).lerp(factor),
            gradient: self
                .gradient
                .as_ref()
                .map(|brush| faded_brush(brush, factor)),
            aligned_gradient: self
                .aligned_gradient
                .as_ref()
                .map(|gradient| gradient.scale(factor)),
            shadows: self
                .shadows
                .iter()
                .map(|shadow| shadow.scale(factor))
                .collect(),
            image: faded_decoration_image(self.image.as_ref(), factor),
            shape: self
                .shape
                .scaled(factor)
                .unwrap_or_else(|| self.shape.clone()),
        }
    }

    /// Border insets to add to the child's layout padding. Logical sides
    /// remain unresolved until the caller supplies a text direction.
    #[must_use]
    pub fn padding(&self) -> EdgeInsetsGeometry {
        self.shape.dimensions_geometry()
    }

    /// Tests the shape in local coordinates rather than its bounding box.
    #[must_use]
    pub fn hit_test(&self, size: Size, position: Offset, direction: Option<TextDirection>) -> bool {
        self.shape.hit_test(
            Rect::from_origin_size(Offset::ZERO, size),
            position,
            direction,
        )
    }

    /// Returns the outer outline used by a decoration clip.
    #[must_use]
    pub fn get_clip_path(
        &self,
        rect: Rect,
        direction: Option<TextDirection>,
    ) -> incular_rendering::Path {
        self.shape.outer_path_with_direction(rect, direction)
    }

    /// Returns the shadow's paint bounds after translation and spread.
    /// Sorts the edges if a negative spread inverts either axis.
    #[must_use]
    pub fn shadow_rect(rect: Rect, shadow: &BoxShadow) -> Rect {
        shadow.paint_bounds(rect)
    }

    /// `_ShapeDecorationPainter.paint` without the shadows and image: the
    /// interior fill over the outer path, then the shape's border.
    #[must_use]
    pub fn paint(&self, size: Size) -> DisplayList {
        let mut canvas = Canvas::default();
        self.paint_fill_into(&mut canvas, size, None);
        self.paint_outline_into(&mut canvas, size, None);
        canvas.finish()
    }

    /// The interior color or gradient, inset behind an opaque outline where
    /// Flutter's specialized solid-fill path avoids antialiasing leaks.
    #[must_use]
    pub fn paint_fill(&self, size: Size) -> DisplayList {
        self.paint_fill_with_direction(size, None)
    }

    fn paint_fill_with_direction(
        &self,
        size: Size,
        direction: Option<TextDirection>,
    ) -> DisplayList {
        let mut canvas = Canvas::default();
        self.paint_fill_into(&mut canvas, size, direction);
        canvas.finish()
    }

    /// The shape's own border, painted last.
    #[must_use]
    pub fn paint_outline(&self, size: Size) -> DisplayList {
        self.paint_outline_with_direction(size, None)
    }

    fn paint_outline_with_direction(
        &self,
        size: Size,
        direction: Option<TextDirection>,
    ) -> DisplayList {
        let mut canvas = Canvas::default();
        self.paint_outline_into(&mut canvas, size, direction);
        canvas.finish()
    }

    fn paint_fill_into(&self, canvas: &mut Canvas, size: Size, direction: Option<TextDirection>) {
        let rect = Rect::from_origin_size(Offset::ZERO, size);
        let brush = self
            .aligned_gradient
            .as_ref()
            .map(|shader| shader.resolve(rect, direction.unwrap_or(TextDirection::Ltr)))
            .or_else(|| self.gradient.clone())
            .or_else(|| self.color.map(Brush::Solid));
        if let Some(brush) = brush {
            // Match _adjustedRectOnOutlinedBorder only for the specialized
            // solid-color interior. The stroke, gradient coordinates, image
            // clip and shadow still use the original box.
            let rect = if self.color.is_some()
                && self.shape.prefer_paint_interior()
                && let Some(outline) = self.shape.rounded_circle_outline()
                && outline.side().style == BorderStyle::Solid
                && outline.side().color.alpha == 255
            {
                let inset = outline.side().stroke_inset() / 2.0;
                let width = rect.size.width - 2.0 * inset;
                let height = rect.size.height - 2.0 * inset;
                if !(width > 0.0 && height > 0.0) {
                    return;
                }
                Rect::from_origin_size(
                    rect.origin + Offset::new(inset, inset),
                    Size::new(width, height),
                )
            } else {
                rect
            };
            self.paint_interior_or_path(canvas, rect, brush, direction);
        }
    }

    fn paint_interior_or_path(
        &self,
        canvas: &mut Canvas,
        rect: Rect,
        brush: Brush,
        direction: Option<TextDirection>,
    ) {
        if self.shape.prefer_paint_interior() {
            self.shape.paint_interior(
                canvas,
                rect,
                &incular_rendering::Paint::new().brush(brush),
                direction,
            );
        } else {
            canvas.fill_path(
                Arc::new(self.shape.outer_path_with_direction(rect, direction)),
                brush,
                FillRule::NonZero,
            );
        }
    }

    fn paint_outline_into(
        &self,
        canvas: &mut Canvas,
        size: Size,
        direction: Option<TextDirection>,
    ) {
        self.shape.paint_with_direction(
            canvas,
            Rect::from_origin_size(Offset::ZERO, size),
            direction,
        );
    }

    /// One shadow's path filled with its color, before the blur.
    fn paint_shadow(
        &self,
        size: Size,
        shadow: &BoxShadow,
        direction: Option<TextDirection>,
    ) -> DisplayList {
        let rect = Self::shadow_rect(Rect::from_origin_size(Offset::ZERO, size), shadow);
        let mut canvas = Canvas::default();
        if rect.size.width > 0.0 && rect.size.height > 0.0 {
            self.paint_interior_or_path(&mut canvas, rect, Brush::Solid(shadow.color), direction);
        }
        canvas.finish()
    }
}

impl Lerp for ShapeDecoration {
    fn lerp(&self, other: &Self, t: f32) -> Self {
        Self::lerp_optional(Some(self), Some(other), t)
            .expect("two present shape decorations")
            .into_owned()
    }
}

/// The color-to-gradient case uses a matching uniform gradient. Compatible
/// rounded and circular shapes morph continuously. Incompatible shape
/// families and mixed relative/absolute gradient coordinates switch at half.
fn lerp_shape_decoration(a: &ShapeDecoration, b: &ShapeDecoration, t: f32) -> ShapeDecoration {
    let switches = t >= 0.5;
    let mixes_coordinates = a.aligned_gradient.is_some() && b.gradient.is_some()
        || b.aligned_gradient.is_some() && a.gradient.is_some();
    let (gradient, aligned_gradient) = if mixes_coordinates {
        let endpoint = if switches { b } else { a };
        (endpoint.gradient.clone(), endpoint.aligned_gradient.clone())
    } else if a.aligned_gradient.is_some() || b.aligned_gradient.is_some() {
        let aligned = match (&a.aligned_gradient, &b.aligned_gradient) {
            (Some(a), Some(b)) => a.lerp(b, t),
            (Some(gradient), None) => match b.color {
                Some(color) => gradient.lerp(&gradient.from_color(color), t),
                None => gradient.scale(1. - t),
            },
            (None, Some(gradient)) => match a.color {
                Some(color) => gradient.from_color(color).lerp(gradient, t),
                None => gradient.scale(t),
            },
            (None, None) => unreachable!("one box-relative shader is present"),
        };
        (None, Some(aligned))
    } else {
        let gradient = match (&a.gradient, &b.gradient) {
            (None, Some(gradient)) => Some(blend_shape_gradient(gradient, a.color, t, true)),
            (Some(gradient), None) => Some(blend_shape_gradient(gradient, b.color, t, false)),
            (a, b) => lerp_raw_brush(a.as_ref(), b.as_ref(), t),
        };
        (gradient, None)
    };
    let shape = lerp_shape_border(Some(&a.shape), Some(&b.shape), t)
        .expect("two present shape outlines");
    ShapeDecoration {
        color: if gradient.is_none() && aligned_gradient.is_none() {
            OptionalColorTween::new(a.color, b.color).lerp(t)
        } else {
            None
        },
        gradient,
        aligned_gradient,
        shadows: BoxShadow::lerp_list(&a.shadows, &b.shadows, t),
        image: DecorationImage::lerp_optional(a.image.as_ref(), b.image.as_ref(), t),
        shape,
    }
}

fn blend_shape_gradient(
    template: &Brush,
    solid: Option<Color>,
    t: f32,
    toward_gradient: bool,
) -> Brush {
    let Some(solid) = solid else {
        return faded_brush(template, if toward_gradient { t } else { 1. - t });
    };
    let stops = |gradient: &GradientStops| {
        let uniform = gradient.with_color(solid);
        if toward_gradient {
            uniform.interpolate(gradient, t)
        } else {
            gradient.interpolate(&uniform, t)
        }
    };
    match template {
        Brush::Solid(color) => {
            let (a, b) = if toward_gradient {
                (Some(solid), Some(*color))
            } else {
                (Some(*color), Some(solid))
            };
            Brush::Solid(
                OptionalColorTween::new(a, b)
                    .lerp(t)
                    .unwrap_or(Color::TRANSPARENT),
            )
        }
        Brush::LinearGradient(gradient) => {
            let mut gradient = gradient.clone();
            gradient.stops = stops(&gradient.stops);
            Brush::LinearGradient(gradient)
        }
        Brush::RadialGradient(gradient) => {
            let mut gradient = gradient.clone();
            gradient.stops = stops(&gradient.stops);
            Brush::RadialGradient(gradient)
        }
        Brush::SweepGradient(gradient) => {
            let mut gradient = gradient.clone();
            gradient.stops = stops(&gradient.stops);
            Brush::SweepGradient(gradient)
        }
    }
}

/// Flutter `DecoratedBox` with a [`ShapeDecoration`] at the child's size.
/// The decoration paints behind the child by default.
#[derive(Clone)]
pub struct DecoratedShapeBox {
    decoration: ShapeDecoration,
    position: DecorationPosition,
    child: Widget,
}

impl DecoratedShapeBox {
    #[must_use]
    pub fn new(decoration: ShapeDecoration, child: impl Into<Widget>) -> Self {
        Self {
            decoration,
            position: DecorationPosition::Background,
            child: child.into(),
        }
    }

    /// Places the decoration before or after the child in paint order.
    #[must_use]
    pub fn position(mut self, position: DecorationPosition) -> Self {
        self.position = position;
        self
    }
}

impl From<DecoratedShapeBox> for Widget {
    fn from(value: DecoratedShapeBox) -> Self {
        use crate::tree::DecorationHitGeometry;
        let decoration = Rc::new(value.decoration);
        let child = value.child;
        let position = value.position;
        let needs_direction = decoration.shape.needs_text_direction()
            || decoration
                .aligned_gradient
                .as_ref()
                .is_some_and(AlignedShapeGradient::needs_text_direction);
        let builder = Builder::new(move |context| {
            let direction = context.text_direction();
            let hit = DecorationHitGeometry::Shape {
                border: decoration.shape.clone(),
                direction,
            };
            let decoration = decoration.clone();
            // Fill the stack at the child's final size; paint layers have no
            // input surface. The outer decoration supplies the shape hit
            // only when the real child does not accept the point.
            let painted = LayoutBuilder::new(move |_, constraints| {
                let size = constraints.biggest();
                let mut layers: Vec<Widget> = Vec::with_capacity(decoration.shadows.len() + 3);
                layers.extend(decoration.shadows.iter().map(|shadow| {
                    Widget::blur(
                        crate::utilities::banner_shadow_sigma(shadow.blur_radius),
                        CustomPaint::new(size, decoration.paint_shadow(size, shadow, direction))
                            .into(),
                    )
                }));
                layers.push(
                    CustomPaint::new(size, decoration.paint_fill_with_direction(size, direction))
                        .into(),
                );
                if let Some(image) = &decoration.image {
                    // The decoration's public clip and hit testing use the
                    // outer contour. Only its image is clipped inside the
                    // border, including every layer of a compound stroke.
                    let clip = Arc::new(decoration.shape.inner_path_with_direction(
                        Rect::from_origin_size(Offset::ZERO, size),
                        direction,
                    ));
                    let painted = super::box_painter::decoration_image_widget(image);
                    layers.push(Positioned::fill(ClipPath::new(clip, painted)).into());
                }
                layers.push(
                    CustomPaint::new(
                        size,
                        decoration.paint_outline_with_direction(size, direction),
                    )
                    .into(),
                );
                Stack::new(layers).clip_behavior(Clip::None).into()
            });
            let layer = Positioned::fill(IgnorePointer::new(painted)).into();
            let stack = Stack::new(match position {
                DecorationPosition::Background => [layer, child.clone()],
                DecorationPosition::Foreground => [child.clone(), layer],
            })
            .fit(StackFit::Passthrough)
            .clip_behavior(Clip::None);
            Widget::decorated_with_geometry(
                None,
                None,
                None,
                CornerRadii::ZERO,
                stack.into(),
                Some(hit),
            )
        });
        if needs_direction {
            builder.require_directionality(&crate::tree::DirectionRequirement {
                widget: "DecoratedShapeBox",
                reason: "No TextDirection found. A logical shape border or shader requires Directionality",
            }).into()
        } else {
            builder.into()
        }
    }
}

/// A custom path clipper that asks a border for its outline at the final
/// layout size, with the supplied text direction.
#[derive(Clone)]
pub struct ShapeBorderClipper {
    pub shape: Rc<dyn ShapeBorder>,
    pub text_direction: Option<TextDirection>,
}

impl ShapeBorderClipper {
    #[must_use]
    pub fn new(shape: Rc<dyn ShapeBorder>, text_direction: Option<TextDirection>) -> Self {
        Self {
            shape,
            text_direction,
        }
    }
}

impl CustomClipper for ShapeBorderClipper {
    fn get_clip(&self, size: Size) -> ClipShape {
        let rect = Rect::from_origin_size(Offset::ZERO, size);
        ClipShape::Path(Arc::new(
            self.shape
                .outer_path_with_direction(rect, self.text_direction),
        ))
    }

    fn should_reclip(&self, old: &Self) -> bool {
        !Rc::ptr_eq(&self.shape, &old.shape) || self.text_direction != old.text_direction
    }
}

impl ClipPath {
    /// Flutter `ClipPath.shape`: clips the child to `shape`'s outer path.
    #[must_use]
    pub fn shape(shape: impl ShapeBorder + 'static, child: impl Into<Widget>) -> Widget {
        Self::shape_with_options(Rc::new(shape), Clip::AntiAlias, Some(child.into()))
    }

    /// Builds a shape clip with an explicit mode and optional child. Keep
    /// the shared shape across rebuilds to retain the resolved outline.
    #[must_use]
    pub fn shape_with_options(
        shape: Rc<dyn ShapeBorder>,
        clip_behavior: Clip,
        child: Option<Widget>,
    ) -> Widget {
        Self::shape_with_hit_policy(shape, clip_behavior, child, true)
    }

    pub(crate) fn shape_with_hit_policy(
        shape: Rc<dyn ShapeBorder>,
        clip_behavior: Clip,
        child: Option<Widget>,
        clip_hit_tests: bool,
    ) -> Widget {
        Builder::new(move |context| {
            let delegate = crate::layout::ClipDelegate::new(ShapeBorderClipper::new(
                shape.clone(),
                context.text_direction(),
            ))
            .with_hit_testing(clip_hit_tests);
            let clip = Self::builder()
                .clip_behavior(clip_behavior)
                .clipper(delegate);
            match &child {
                Some(child) => clip.child(child.clone()).build().into(),
                None => clip.build().into(),
            }
        })
        .into()
    }
}
