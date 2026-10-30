//! Flutter's `_BoxDecorationPainter` (`painting/box_decoration.dart`),
//! `Border.paint` (`painting/box_border.dart`) and `DecorationPosition`:
//! every part of a [`BoxDecoration`] painted at the decorated box's size,
//! behind or in front of the child.

use std::{rc::Rc, sync::Arc};

use incular_config::EdgeInsets;
use incular_core::{Color, Offset, Rect, Size};
use incular_rendering::{Brush, Canvas, CornerRadii, DisplayList, FillRule, Path, Stroke};

use super::decoration::{Border, BorderSide, BorderStyle, BoxDecoration, BoxShadow, BoxShape};
use super::geometry::{BorderRadius, Radius};
use crate::{
    ClipPath, CustomPaint, IgnorePointer, Image, LayoutBuilder, Opacity, Positioned, Stack,
    StackFit, Widget,
};

/// Flutter `DecorationPosition`: whether a decoration paints behind or in
/// front of the child.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum DecorationPosition {
    #[default]
    Background,
    Foreground,
}

impl BoxDecoration {
    /// Whether painting this decoration needs more than a fill and one
    /// uniform inside border: shadows, a circle, an image, or a border
    /// whose sides differ or are not drawn inside the box.
    #[must_use]
    pub(crate) fn needs_full_painter(&self) -> bool {
        !self.box_shadow.is_empty()
            || self.shape == BoxShape::Circle
            || self.image.is_some()
            || self
                .border
                .is_some_and(|border| !border.is_uniform() || border.top.stroke_align != -1.0)
    }

    /// Flutter `BoxDecoration.padding`: the border's `dimensions`.
    #[must_use]
    pub(crate) fn padding(&self) -> EdgeInsets {
        self.border
            .map_or(EdgeInsets::ZERO, |border| border.stroke_insets())
    }

    /// `_BoxDecorationPainter._paintShadows` for one shadow: its box,
    /// shifted and inflated, filled with its color before the blur.
    #[must_use]
    pub fn paint_shadow(&self, size: Size, shadow: &BoxShadow) -> DisplayList {
        let rect = Rect::from_origin_size(Offset::ZERO, size);
        let bounds = inflate(
            Rect::from_origin_size(Offset::new(shadow.offset.x, shadow.offset.y), rect.size),
            shadow.spread_radius,
        );
        let mut canvas = Canvas::default();
        canvas.fill_path(
            Arc::new(self.box_path(bounds)),
            shadow.color,
            FillRule::NonZero,
        );
        canvas.finish()
    }

    /// `_BoxDecorationPainter._paintBackgroundColor`: the color or
    /// gradient over the box, pulled in by half of each opaque solid side's
    /// stroke inset. `None` without a color or gradient.
    #[must_use]
    pub fn paint_background(&self, size: Size) -> Option<DisplayList> {
        let brush = self
            .gradient
            .clone()
            .or_else(|| self.color.map(Brush::Solid))?;
        let rect = Rect::from_origin_size(Offset::ZERO, size);
        let adjusted = match self.border {
            Some(border) => {
                let adjusted_side = |side: BorderSide| {
                    if side.color.alpha == 255 && side.style == BorderStyle::Solid {
                        side.stroke_inset() / 2.0
                    } else {
                        0.0
                    }
                };
                deflate_sides(
                    rect,
                    EdgeInsets::only(
                        adjusted_side(border.left),
                        adjusted_side(border.top),
                        adjusted_side(border.right),
                        adjusted_side(border.bottom),
                    ),
                )
            }
            None => rect,
        };
        let mut canvas = Canvas::default();
        canvas.fill_path(Arc::new(self.box_path(adjusted)), brush, FillRule::NonZero);
        Some(canvas.finish())
    }

    /// Flutter `Border.paint` for this decoration's shape and radius.
    /// `None` without a border.
    #[must_use]
    pub fn paint_border(&self, size: Size) -> Option<DisplayList> {
        let border = self.border?;
        let rect = Rect::from_origin_size(Offset::ZERO, size);
        let mut canvas = Canvas::default();
        let radius = self
            .border_radius
            .filter(|radius| *radius != BorderRadius::default());
        if border.is_uniform() {
            let side = border.top;
            if side.style == BorderStyle::None {
                return None;
            }
            match (self.shape, radius) {
                (BoxShape::Circle, _) => {
                    // `_paintUniformBorderWithCircle`.
                    let radius = (shortest_side(rect) + side.stroke_offset()) / 2.0;
                    stroke(&mut canvas, &side, circle_path(center(rect), radius));
                }
                (BoxShape::Rectangle, Some(radius)) => {
                    // `_paintUniformBorderWithRadius`.
                    if side.width == 0.0 {
                        stroke(&mut canvas, &side, rrect_path(rect, radius));
                    } else {
                        let outer =
                            inflate_rrect(rect, radius, EdgeInsets::all(side.stroke_outset()));
                        let inner =
                            deflate_rrect(rect, radius, EdgeInsets::all(side.stroke_inset()));
                        fill_between(&mut canvas, outer, inner, side.color);
                    }
                }
                (BoxShape::Rectangle, None) => {
                    // `_paintUniformBorderWithRectangle`.
                    let path = rect_path(inflate(rect, side.stroke_offset() / 2.0));
                    stroke(&mut canvas, &side, path);
                }
            }
            return Some(canvas.finish());
        }
        let sides = [border.top, border.right, border.bottom, border.left];
        if sides.iter().all(|side| side.style == BorderStyle::None) {
            return None;
        }
        let mut visible = sides
            .iter()
            .filter(|side| side.style != BorderStyle::None)
            .map(|side| side.color);
        let first = visible.next().expect("a side is visible");
        let one_color = visible.all(|color| color == first);
        let hairline = sides
            .iter()
            .any(|side| side.style == BorderStyle::Solid && side.width == 0.0);
        if one_color && !hairline && (self.shape == BoxShape::Circle || radius.is_some()) {
            // `BoxBorder.paintNonUniformBorder`.
            let (box_rect, radius) = match self.shape {
                BoxShape::Circle => (
                    circle_rect(center(rect), shortest_side(rect) / 2.0),
                    BorderRadius::all(Radius::circular(rect.size.width)),
                ),
                BoxShape::Rectangle => (rect, radius.unwrap_or_default()),
            };
            let visible_side = |side: BorderSide| {
                if side.style == BorderStyle::None {
                    BorderSide::NONE
                } else {
                    side
                }
            };
            let [top, right, bottom, left] = sides.map(visible_side);
            let inner = deflate_rrect(
                box_rect,
                radius,
                EdgeInsets::only(
                    left.stroke_inset(),
                    top.stroke_inset(),
                    right.stroke_inset(),
                    bottom.stroke_inset(),
                ),
            );
            let outer = inflate_rrect(
                box_rect,
                radius,
                EdgeInsets::only(
                    left.stroke_outset(),
                    top.stroke_outset(),
                    right.stroke_outset(),
                    bottom.stroke_outset(),
                ),
            );
            fill_between(&mut canvas, outer, inner, first);
            return Some(canvas.finish());
        }
        paint_border_sides(&mut canvas, rect, border);
        Some(canvas.finish())
    }

    /// `_BoxDecorationPainter._paintBox`'s geometry: a circle inscribed in
    /// the shortest side, or the rectangle with its corner radii.
    fn box_path(&self, rect: Rect) -> Path {
        match self.shape {
            BoxShape::Circle => circle_path(center(rect), shortest_side(rect) / 2.0),
            BoxShape::Rectangle => match self.border_radius {
                Some(radius) if radius != BorderRadius::default() => rrect_path(rect, radius),
                _ => rect_path(rect),
            },
        }
    }

    /// `_paintBackgroundImage`'s clip: the circle, or the rounded rectangle
    /// when there is a radius.
    fn image_clip(&self, rect: Rect) -> Option<Path> {
        match self.shape {
            BoxShape::Circle => Some(circle_path(center(rect), shortest_side(rect) / 2.0)),
            BoxShape::Rectangle => self.border_radius.map(|radius| rrect_path(rect, radius)),
        }
    }
}

/// Flutter `DecoratedBox` with a [`BoxDecoration`] and a
/// [`DecorationPosition`]: the decoration paints at the child's size, its
/// shadows blurred by `BoxShadow.blurSigma`.
///
/// Two parts differ from Flutter's painter: every shadow blurs as
/// [`BlurStyle::Normal`](super::decoration::BlurStyle::Normal), and the
/// background blend mode and an image's `centerSlice` are not applied.
pub(crate) fn decorated(
    decoration: BoxDecoration,
    position: DecorationPosition,
    child: Widget,
) -> Widget {
    let decoration = Rc::new(decoration);
    // Filling the stack, which takes the child's size, the builder sees
    // tight constraints of exactly that size.
    let painted = LayoutBuilder::new(move |_, constraints| {
        let size = constraints.biggest();
        let mut layers: Vec<Widget> = decoration
            .box_shadow
            .iter()
            .map(|shadow| {
                Widget::blur(
                    crate::utilities::banner_shadow_sigma(shadow.blur_radius),
                    CustomPaint::new(size, decoration.paint_shadow(size, shadow)).into(),
                )
            })
            .collect();
        if let Some(background) = decoration.paint_background(size) {
            layers.push(CustomPaint::new(size, background).into());
        }
        if let Some(image) = &decoration.image {
            let mut painted: Widget = Image::new(image.image.clone())
                .fit(image.fit)
                .alignment(image.alignment)
                .repeat(image.repeat)
                .scale(image.scale)
                .filter_quality(image.filter_quality)
                .into();
            if image.opacity < 1.0 {
                painted = Opacity::new(image.opacity, painted).into();
            }
            if let Some(clip) = decoration.image_clip(Rect::from_origin_size(Offset::ZERO, size)) {
                painted = ClipPath::new(Arc::new(clip), painted).into();
            }
            layers.push(Positioned::fill(painted).into());
        }
        if let Some(border) = decoration.paint_border(size) {
            layers.push(CustomPaint::new(size, border).into());
        }
        Stack::new(layers).into()
    });
    // The decoration never takes a hit from the child it decorates.
    let layer: Widget = Positioned::fill(IgnorePointer::new(painted)).into();
    let children = match position {
        DecorationPosition::Background => [layer, child],
        DecorationPosition::Foreground => [child, layer],
    };
    Stack::new(children).fit(StackFit::Passthrough).into()
}

/// Flutter `paintBorder`: each solid side as a trapezoid meeting its
/// neighbours on the diagonals, or a hairline for a zero width.
fn paint_border_sides(canvas: &mut Canvas, rect: Rect, border: Border) {
    let (left, top) = (rect.origin.x, rect.origin.y);
    let (right, bottom) = (left + rect.size.width, top + rect.size.height);
    let Border {
        top: t,
        right: r,
        bottom: b,
        left: l,
    } = border;
    let sides = [
        (
            t,
            [
                Offset::new(left, top),
                Offset::new(right, top),
                Offset::new(right - r.width, top + t.width),
                Offset::new(left + l.width, top + t.width),
            ],
        ),
        (
            r,
            [
                Offset::new(right, top),
                Offset::new(right, bottom),
                Offset::new(right - r.width, bottom - b.width),
                Offset::new(right - r.width, top + t.width),
            ],
        ),
        (
            b,
            [
                Offset::new(right, bottom),
                Offset::new(left, bottom),
                Offset::new(left + l.width, bottom - b.width),
                Offset::new(right - r.width, bottom - b.width),
            ],
        ),
        (
            l,
            [
                Offset::new(left, bottom),
                Offset::new(left, top),
                Offset::new(left + l.width, top + t.width),
                Offset::new(left + l.width, bottom - b.width),
            ],
        ),
    ];
    for (side, points) in sides {
        if side.style == BorderStyle::None {
            continue;
        }
        let mut builder = Path::builder();
        builder.move_to(points[0]).line_to(points[1]);
        if side.width == 0.0 {
            stroke(canvas, &side, builder.build());
        } else {
            builder.line_to(points[2]).line_to(points[3]).close();
            canvas.fill_path(Arc::new(builder.build()), side.color, FillRule::NonZero);
        }
    }
}

/// `BorderSide.toPaint` applied to a path.
fn stroke(canvas: &mut Canvas, side: &BorderSide, path: Path) {
    canvas.stroke_path(
        Arc::new(path),
        side.color,
        Stroke {
            width: side.width,
            ..Stroke::default()
        },
    );
}

/// `Canvas.drawDRRect`: the area inside `outer` and outside `inner`.
fn fill_between(canvas: &mut Canvas, outer: Path, inner: Path, color: Color) {
    let mut builder = Path::builder();
    builder.append(&outer).append(&inner);
    canvas.fill_path(Arc::new(builder.build()), color, FillRule::EvenOdd);
}

fn center(rect: Rect) -> Offset {
    Offset::new(
        rect.origin.x + rect.size.width / 2.0,
        rect.origin.y + rect.size.height / 2.0,
    )
}

fn shortest_side(rect: Rect) -> f32 {
    rect.size.width.min(rect.size.height)
}

/// Flutter `Rect.fromCircle`.
fn circle_rect(center: Offset, radius: f32) -> Rect {
    Rect::from_origin_size(
        Offset::new(center.x - radius, center.y - radius),
        Size::new(2.0 * radius, 2.0 * radius),
    )
}

fn circle_path(center: Offset, radius: f32) -> Path {
    let mut builder = Path::builder();
    builder.rounded_rect(circle_rect(center, radius), CornerRadii::uniform(radius));
    builder.build()
}

fn rect_path(rect: Rect) -> Path {
    let mut builder = Path::builder();
    builder.rounded_rect(rect, CornerRadii::ZERO);
    builder.build()
}

/// Flutter `Rect.inflate`; a negative `delta` deflates.
fn inflate(rect: Rect, delta: f32) -> Rect {
    inflate_sides(rect, EdgeInsets::all(delta))
}

/// Flutter `EdgeInsets.inflateRect`.
fn inflate_sides(rect: Rect, insets: EdgeInsets) -> Rect {
    Rect::from_origin_size(
        Offset::new(rect.origin.x - insets.left, rect.origin.y - insets.top),
        Size::new(
            rect.size.width + insets.left + insets.right,
            rect.size.height + insets.top + insets.bottom,
        ),
    )
}

/// Flutter `EdgeInsets.deflateRect`.
fn deflate_sides(rect: Rect, insets: EdgeInsets) -> Rect {
    inflate_sides(
        rect,
        EdgeInsets::only(-insets.left, -insets.top, -insets.right, -insets.bottom),
    )
}

/// `BorderRadius.toRRect(rect)` as a path with elliptical corners.
fn rrect_path(rect: Rect, radius: BorderRadius) -> Path {
    elliptical_rrect_path(
        rect,
        [
            radius.top_left,
            radius.top_right,
            radius.bottom_right,
            radius.bottom_left,
        ],
    )
}

/// Flutter `EdgeInsets.inflateRRect`: the rectangle grows by `insets` and
/// each corner radius by the two sides that meet there.
fn inflate_rrect(rect: Rect, radius: BorderRadius, insets: EdgeInsets) -> Path {
    let grow = |corner: Radius, x: f32, y: f32| Radius {
        x: (corner.x + x).max(0.0),
        y: (corner.y + y).max(0.0),
    };
    elliptical_rrect_path(
        inflate_sides(rect, insets),
        [
            grow(radius.top_left, insets.left, insets.top),
            grow(radius.top_right, insets.right, insets.top),
            grow(radius.bottom_right, insets.right, insets.bottom),
            grow(radius.bottom_left, insets.left, insets.bottom),
        ],
    )
}

/// Flutter `EdgeInsets.deflateRRect`.
fn deflate_rrect(rect: Rect, radius: BorderRadius, insets: EdgeInsets) -> Path {
    inflate_rrect(
        rect,
        radius,
        EdgeInsets::only(-insets.left, -insets.top, -insets.right, -insets.bottom),
    )
}

/// A closed rounded rectangle with elliptical corners (top-left first,
/// clockwise), each corner a quarter conic. Radii that overflow a side are
/// scaled down together, as Skia's `SkRRect` does.
fn elliptical_rrect_path(rect: Rect, radii: [Radius; 4]) -> Path {
    let (left, top) = (rect.origin.x, rect.origin.y);
    let (width, height) = (rect.size.width.max(0.0), rect.size.height.max(0.0));
    let (right, bottom) = (left + width, top + height);
    let [tl, tr, br, bl] = radii.map(|r| (r.x.max(0.0), r.y.max(0.0)));
    let mut scale: f32 = 1.0;
    for (sum, limit) in [
        (tl.0 + tr.0, width),
        (bl.0 + br.0, width),
        (tl.1 + bl.1, height),
        (tr.1 + br.1, height),
    ] {
        if sum > limit && sum > 0.0 {
            scale = scale.min(limit / sum);
        }
    }
    let [tl, tr, br, bl] = [tl, tr, br, bl].map(|(x, y)| (x * scale, y * scale));
    let weight = std::f32::consts::FRAC_1_SQRT_2;
    let mut builder = Path::builder();
    builder.move_to(Offset::new(left + tl.0, top));
    builder.line_to(Offset::new(right - tr.0, top));
    builder.conic_to(
        Offset::new(right, top),
        Offset::new(right, top + tr.1),
        weight,
    );
    builder.line_to(Offset::new(right, bottom - br.1));
    builder.conic_to(
        Offset::new(right, bottom),
        Offset::new(right - br.0, bottom),
        weight,
    );
    builder.line_to(Offset::new(left + bl.0, bottom));
    builder.conic_to(
        Offset::new(left, bottom),
        Offset::new(left, bottom - bl.1),
        weight,
    );
    builder.line_to(Offset::new(left, top + tl.1));
    builder.conic_to(
        Offset::new(left, top),
        Offset::new(left + tl.0, top),
        weight,
    );
    builder.close();
    builder.build()
}
