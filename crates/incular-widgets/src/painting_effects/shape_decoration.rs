//! Flutter's `ShapeDecoration` (`painting/shape_decoration.dart`) and
//! `ClipPath.shape` (`ShapeBorderClipper`): a [`ShapeBorder`] used as a
//! decoration and as a clip.

use std::{rc::Rc, sync::Arc};

use incular_core::{Color, Offset, Rect, Size};
use incular_rendering::{Brush, Canvas, DisplayList, FillRule};

use super::decoration::BoxShadow;
use super::shape_borders::ShapeBorder;
use crate::{ClipPath, CustomPaint, LayoutBuilder, Positioned, Stack, StackFit, Widget};

/// Flutter `ShapeDecoration`: a shape's interior filled with a color or
/// gradient, its shadows, and the shape's own border on top.
#[derive(Clone)]
pub struct ShapeDecoration {
    pub color: Option<Color>,
    /// Takes precedence over [`ShapeDecoration::color`], as in Flutter.
    pub gradient: Option<Brush>,
    pub shadows: Vec<BoxShadow>,
    pub shape: Rc<dyn ShapeBorder>,
}

impl ShapeDecoration {
    #[must_use]
    pub fn new(shape: impl ShapeBorder + 'static) -> Self {
        Self {
            color: None,
            gradient: None,
            shadows: Vec::new(),
            shape: Rc::new(shape),
        }
    }

    #[must_use]
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    #[must_use]
    pub fn gradient(mut self, gradient: impl Into<Brush>) -> Self {
        self.gradient = Some(gradient.into());
        self
    }

    #[must_use]
    pub fn shadows(mut self, shadows: impl IntoIterator<Item = BoxShadow>) -> Self {
        self.shadows = shadows.into_iter().collect();
        self
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

    /// `_ShapeDecorationPainter.paint` without the shadows: the interior
    /// fill over the outer path, then the shape's border.
    #[must_use]
    pub fn paint(&self, size: Size) -> DisplayList {
        let rect = Rect::from_origin_size(Offset::ZERO, size);
        let mut canvas = Canvas::default();
        let brush = self
            .gradient
            .clone()
            .or_else(|| self.color.map(Brush::Solid));
        if let Some(brush) = brush {
            canvas.fill_path(
                Arc::new(self.shape.outer_path(rect)),
                brush,
                FillRule::NonZero,
            );
        }
        self.shape.paint(&mut canvas, rect);
        canvas.finish()
    }

    /// One shadow's path filled with its color, before the blur.
    fn paint_shadow(&self, size: Size, shadow: &BoxShadow) -> DisplayList {
        let rect = Self::shadow_rect(Rect::from_origin_size(Offset::ZERO, size), shadow);
        let mut canvas = Canvas::default();
        canvas.fill_path(
            Arc::new(self.shape.outer_path(rect)),
            shadow.color,
            FillRule::NonZero,
        );
        canvas.finish()
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
        let decoration = Rc::new(value.decoration);
        // Filling the stack, which takes the child's size, the builder sees
        // tight constraints of exactly that size.
        let painted = LayoutBuilder::new(move |_, constraints| {
            let size = constraints.biggest();
            let mut layers: Vec<Widget> = decoration
                .shadows
                .iter()
                .map(|shadow| {
                    Widget::blur(
                        crate::utilities::banner_shadow_sigma(shadow.blur_radius),
                        CustomPaint::new(size, decoration.paint_shadow(size, shadow)).into(),
                    )
                })
                .collect();
            layers.push(CustomPaint::new(size, decoration.paint(size)).into());
            Stack::new(layers).into()
        });
        Stack::new([Positioned::fill(painted).into(), value.child])
            .fit(StackFit::Passthrough)
            .into()
    }
}

impl ClipPath {
    /// Flutter `ClipPath.shape`: clips the child to `shape`'s outer path.
    ///
    /// The path is built for the largest size the incoming constraints
    /// allow, which is the child's size under tight constraints.
    #[must_use]
    pub fn shape(shape: impl ShapeBorder + 'static, child: impl Into<Widget>) -> Widget {
        let shape: Rc<dyn ShapeBorder> = Rc::new(shape);
        let child = child.into();
        LayoutBuilder::new(move |_, constraints| {
            let rect = Rect::from_origin_size(Offset::ZERO, constraints.biggest());
            ClipPath::new(Arc::new(shape.outer_path(rect)), child.clone()).into()
        })
        .into()
    }
}
