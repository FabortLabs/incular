//! Flutter's `ShapeDecoration` (`painting/shape_decoration.dart`) and
//! `ClipPath.shape` (`ShapeBorderClipper`): a [`ShapeBorder`] used as a
//! decoration and as a clip.

use std::{borrow::Cow, rc::Rc, sync::Arc};

use incular_animation::OptionalColorTween;
use incular_config::{Clip, TextDirection};
use incular_core::{Color, Lerp, Offset, Rect, Size};
use incular_rendering::{Brush, Canvas, CornerRadii, DisplayList, FillRule, GradientStops};

use super::decoration::{BoxShadow, DecorationImage, faded_brush, faded_decoration_image};
use super::shape_borders::ShapeBorder;
use crate::{
    Builder, ClipPath, ClipShape, CustomClipper, CustomPaint, IgnorePointer, LayoutBuilder,
    Positioned, Stack, StackFit, Widget,
};

/// Flutter `ShapeDecoration`: a shape's interior filled with a color or
/// gradient, an image clipped to the shape, its shadows, and the shape's own
/// border on top.
#[derive(Clone)]
pub struct ShapeDecoration {
    pub color: Option<Color>,
    /// A gradient fill is mutually exclusive with [`ShapeDecoration::color`].
    pub gradient: Option<Brush>,
    pub shadows: Vec<BoxShadow>,
    /// Flutter `ShapeDecoration.image`: painted over the color or gradient,
    /// clipped to the shape's outer path, under the border.
    pub image: Option<DecorationImage>,
    pub shape: Rc<dyn ShapeBorder>,
}

impl ShapeDecoration {
    #[must_use]
    pub fn new(shape: impl ShapeBorder + 'static) -> Self {
        Self {
            color: None,
            gradient: None,
            shadows: Vec::new(),
            image: None,
            shape: Rc::new(shape),
        }
    }

    #[must_use]
    pub fn color(mut self, color: Color) -> Self {
        debug_assert!(
            self.gradient.is_none(),
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

    /// Flutter `BoxShadow.toPaint`'s path: the outline of the shape in
    /// `rect` shifted by the shadow's offset and inflated by its spread.
    #[must_use]
    pub fn shadow_rect(rect: Rect, shadow: &BoxShadow) -> Rect {
        let spread = shadow.spread_radius;
        Rect::from_origin_size(
            Offset::new(
                rect.origin.x + shadow.offset.x - spread,
                rect.origin.y + shadow.offset.y - spread,
            ),
            Size::new(
                rect.size.width + 2.0 * spread,
                rect.size.height + 2.0 * spread,
            ),
        )
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

    /// The interior color or gradient over the outer path.
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
            .gradient
            .clone()
            .or_else(|| self.color.map(Brush::Solid));
        if let Some(brush) = brush {
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
        self.paint_interior_or_path(&mut canvas, rect, Brush::Solid(shadow.color), direction);
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
/// rounded and circular shapes morph continuously; unrelated families,
/// pairs of distinct images and pairs of distinct gradients use a midpoint
/// fallback.
fn lerp_shape_decoration(a: &ShapeDecoration, b: &ShapeDecoration, t: f32) -> ShapeDecoration {
    let switches = t >= 0.5;
    let gradient = match (&a.gradient, &b.gradient) {
        (None, Some(gradient)) => Some(blend_shape_gradient(gradient, a.color, t, true)),
        (Some(gradient), None) => Some(blend_shape_gradient(gradient, b.color, t, false)),
        (a, b) => {
            if switches {
                b.clone()
            } else {
                a.clone()
            }
        }
    };
    let shape = if Rc::ptr_eq(&a.shape, &b.shape) {
        a.shape.clone()
    } else if let Some(outline) = a.shape.rounded_circle_outline().and_then(|a| {
        let b = b.shape.rounded_circle_outline()?;
        a.try_interpolate(b, t)
    }) {
        Rc::new(outline)
    } else if switches {
        b.shape.clone()
    } else {
        a.shape.clone()
    };
    ShapeDecoration {
        color: if gradient.is_none() {
            OptionalColorTween::new(a.color, b.color).lerp(t)
        } else {
            None
        },
        gradient,
        shadows: BoxShadow::lerp_list(&a.shadows, &b.shadows, t),
        image: match (&a.image, &b.image) {
            (Some(a), None) => faded_decoration_image(Some(a), 1. - t),
            (None, Some(b)) => faded_decoration_image(Some(b), t),
            (a, b) => {
                if switches {
                    b.clone()
                } else {
                    a.clone()
                }
            }
        },
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

/// Flutter `DecoratedBox` with a [`ShapeDecoration`]: the decoration paints
/// behind the child, at the child's size.
#[derive(Clone)]
pub struct DecoratedShapeBox {
    decoration: ShapeDecoration,
    child: Widget,
}

impl DecoratedShapeBox {
    #[must_use]
    pub fn new(decoration: ShapeDecoration, child: impl Into<Widget>) -> Self {
        Self {
            decoration,
            child: child.into(),
        }
    }
}

impl From<DecoratedShapeBox> for Widget {
    fn from(value: DecoratedShapeBox) -> Self {
        use crate::tree::DecorationHitGeometry;
        let decoration = Rc::new(value.decoration);
        let child = value.child;
        Builder::new(move |context| {
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
                    let clip = Arc::new(
                        decoration
                            .get_clip_path(Rect::from_origin_size(Offset::ZERO, size), direction),
                    );
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
            let stack = Stack::new([
                Positioned::fill(IgnorePointer::new(painted)).into(),
                child.clone(),
            ])
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
        })
        .into()
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
