//! Flutter's `_BoxDecorationPainter` (`painting/box_decoration.dart`),
//! `Border.paint` (`painting/box_border.dart`) and `DecorationPosition`:
//! every part of a [`BoxDecoration`] painted at the decorated box's size,
//! behind or in front of the child.

use std::{rc::Rc, sync::Arc};

use incular_config::{Clip, EdgeInsets};
use incular_core::{Color, Offset, Rect, Size};
use incular_rendering::{Brush, Canvas, CornerRadii, DisplayList, FillRule, Path, RRect, Stroke};

use super::decoration::{
    Border, BorderSide, BorderStyle, BoxDecoration, BoxShadow, BoxShape, DecorationImage,
};
use super::geometry::{BorderRadius, Radius};
use crate::{
    ClipPath, ClipRRect, ClipRect, ClipShape, CustomClipper, CustomPaint, IgnorePointer, Image,
    LayoutBuilder, Opacity, Positioned, Stack, StackFit, Widget,
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
            || self.background_blend_mode.is_some()
            || self.shape == BoxShape::Circle
            || self.image.is_some()
            || self.has_elliptical_corners()
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
        self.fill_box(&mut canvas, bounds, Brush::Solid(shadow.color));
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
        self.fill_box(&mut canvas, adjusted, brush);
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
    /// Fills the decoration's shape over `rect`. Circles and rectangles with
    /// circular corners paint as rounded rectangles, which renderers draw
    /// analytically as Skia's `drawCircle` / `drawRRect` do; elliptical
    /// corners fall back to a path.
    fn fill_box(&self, canvas: &mut Canvas, rect: Rect, brush: Brush) {
        match self.rounded_box(rect) {
            Some(rrect) => canvas.rrect(rrect, brush),
            None => canvas.fill_path(Arc::new(self.clip_path(rect)), brush, FillRule::NonZero),
        }
    }

    fn has_elliptical_corners(&self) -> bool {
        self.shape == BoxShape::Rectangle
            && self.border_radius.is_some_and(|radius| {
                clockwise_radii(radius)
                    .iter()
                    .any(|radius| radius.x != radius.y)
            })
    }

    /// Clip the child, not the container's foreground, background, or shadow.
    pub(crate) fn clip_child(&self, mode: Clip, child: Widget) -> Widget {
        if self.shape == BoxShape::Rectangle
            && self.border_radius.unwrap_or_default() == BorderRadius::default()
        {
            return ClipRect::new(child).clip_behavior(mode).into();
        }
        let clipper = BoxDecorationClipper(BoxDecoration {
            shape: self.shape,
            border_radius: self.border_radius,
            ..BoxDecoration::default()
        });
        if self.has_elliptical_corners() {
            ClipPath::builder()
                .clip_behavior(mode)
                .child(child)
                .build()
                .custom_clipper(clipper)
                .into()
        } else {
            ClipRRect::new(CornerRadii::ZERO, child)
                .clip_behavior(mode)
                .custom_clipper(clipper)
                .into()
        }
    }

    fn rounded_box(&self, rect: Rect) -> Option<RRect> {
        match self.shape {
            BoxShape::Circle => {
                let side = shortest_side(rect);
                let centre = center(rect);
                Some(RRect {
                    rect: Rect::from_origin_size(
                        Offset::new(centre.x - side / 2.0, centre.y - side / 2.0),
                        Size::new(side, side),
                    ),
                    radii: CornerRadii::uniform(side / 2.0),
                })
            }
            BoxShape::Rectangle => {
                let radius = self.border_radius.unwrap_or_default();
                let corners = [
                    radius.top_left,
                    radius.top_right,
                    radius.bottom_right,
                    radius.bottom_left,
                ];
                if corners.iter().any(|corner| corner.x != corner.y) {
                    return None;
                }
                let [tl, tr, br, bl] = corners.map(|corner| corner.x.max(0.0));
                // SkRRect::scaleRadii: shrink every radius by one factor
                // until adjacent corners fit along each side.
                let fit = |extent: f32, sum: f32| if sum > extent { extent / sum } else { 1.0 };
                let scale = fit(rect.size.width, tl + tr)
                    .min(fit(rect.size.width, bl + br))
                    .min(fit(rect.size.height, tl + bl))
                    .min(fit(rect.size.height, tr + br));
                Some(RRect {
                    rect,
                    radii: CornerRadii {
                        top_left: tl * scale,
                        top_right: tr * scale,
                        bottom_right: br * scale,
                        bottom_left: bl * scale,
                    },
                })
            }
        }
    }

    /// Returns the outline of the decoration in `rect`. The border and
    /// shadows do not change the clip; a circle uses the shortest side.
    #[must_use]
    pub fn clip_path(&self, rect: Rect) -> Path {
        match self.shape {
            BoxShape::Circle => circle_path(center(rect), shortest_side(rect) / 2.0),
            BoxShape::Rectangle => match self.border_radius {
                Some(radius) if radius != BorderRadius::default() => rrect_path(rect, radius),
                _ => rect_path(rect),
            },
        }
    }

    /// Tests the decoration's shape at a position local to the box. This
    /// tests geometry only, regardless of color, borders, or shadows.
    #[must_use]
    pub fn hit_test(&self, size: Size, position: Offset) -> bool {
        let bounds = Rect::from_origin_size(Offset::ZERO, size);
        let (x, y) = (position.x, position.y);
        if !((0.0..size.width).contains(&x) && (0.0..size.height).contains(&y)) {
            return false;
        }
        match self.shape {
            BoxShape::Circle => {
                let delta = position - center(bounds);
                let radius = shortest_side(bounds) / 2.0;
                delta.x.mul_add(delta.x, delta.y * delta.y) <= radius * radius
            }
            BoxShape::Rectangle => self.border_radius.is_none_or(|radius| {
                let [tl, tr, br, bl] = normalized_radii(size, clockwise_radii(radius));
                let (right, bottom) = (size.width - x, size.height - y);
                [
                    (x, y, tl),
                    (right, y, tr),
                    (right, bottom, br),
                    (x, bottom, bl),
                ]
                .into_iter()
                .find(|&(x, y, (rx, ry))| x < rx && y < ry)
                .is_none_or(|(x, y, (rx, ry))| {
                    let dx = (rx - x) / rx;
                    let dy = (ry - y) / ry;
                    dx.mul_add(dx, dy * dy) <= 1.0
                })
            }),
        }
    }

    /// `_paintBackgroundImage`'s clip: the circle, or the rounded rectangle
    /// when there is a radius.
    fn image_clip(&self, rect: Rect) -> Option<Path> {
        match self.shape {
            BoxShape::Circle => Some(self.clip_path(rect)),
            BoxShape::Rectangle => self.border_radius.map(|_| self.clip_path(rect)),
        }
    }
}

struct BoxDecorationClipper(BoxDecoration);

impl CustomClipper for BoxDecorationClipper {
    fn get_clip(&self, size: Size) -> ClipShape {
        let rect = Rect::from_origin_size(Offset::ZERO, size);
        self.0.rounded_box(rect).map_or_else(
            || ClipShape::Path(Arc::new(self.0.clip_path(rect))),
            ClipShape::RRect,
        )
    }

    fn should_reclip(&self, old: &Self) -> bool {
        self.0.shape != old.0.shape || self.0.border_radius != old.0.border_radius
    }
}

/// The image stage of a decoration: the image fitted with the resolved fit,
/// alignment, repeat, scale, sampling and opacity. The caller clips it.
pub(crate) fn decoration_image_widget(image: &DecorationImage) -> Widget {
    let mut painted: Widget = Image::new(image.image.clone())
        .fit(image.resolved_fit())
        .alignment(image.alignment)
        .repeat(image.repeat)
        .scale(image.scale)
        .filter_quality(image.filter_quality)
        .into();
    if image.opacity < 1.0 {
        painted = Opacity::new(image.opacity, painted).into();
    }
    painted
}

/// Flutter `DecoratedBox` with a [`BoxDecoration`] and a
/// [`DecorationPosition`]: the decoration paints at the child's size, its
/// shadows blurred by `BoxShadow.blurSigma`.
///
/// Shadows follow their `blurStyle`, the background fill takes the
/// background blend mode, and the image nine-slices with `centerSlice` and
/// mirrors under a right-to-left [`Directionality`](crate::Directionality)
/// with `matchTextDirection`.
pub(crate) fn decorated(
    decoration: BoxDecoration,
    position: DecorationPosition,
    child: Widget,
) -> Widget {
    let decoration = Rc::new(decoration);
    let hit_decoration = decoration.clone();
    // Filling the stack, which takes the child's size, the builder sees
    // tight constraints of exactly that size.
    let painted = LayoutBuilder::new(move |context, constraints| {
        let size = constraints.biggest();
        // Shadows plus at most three layers pushed below: background, image, border.
        let mut layers = Vec::with_capacity(decoration.box_shadow.len() + 3);
        layers.extend(
            decoration
                .box_shadow
                .iter()
                .map(|shadow| styled_shadow(&decoration, size, shadow)),
        );
        if let Some(background) = decoration.paint_background(size) {
            let background: Widget = CustomPaint::new(size, background).into();
            // Flutter `_getBackgroundPaint`: the fill alone takes the
            // background blend mode, against what is painted below it.
            layers.push(match decoration.background_blend_mode {
                Some(mode) => Widget::blend(mode, background),
                None => background,
            });
        }
        if let Some(image) = &decoration.image {
            // `DecorationImagePainter.paint`: flip when the image matches
            // an ambient right-to-left direction.
            let flip = image.match_text_direction
                && context.text_direction() == Some(incular_config::TextDirection::Rtl);
            let mut painted = if image.center_slice.is_some() || flip {
                let paint = ImagePaint {
                    image: image.image.clone(),
                    scale: image.scale,
                    fit: image.fit,
                    alignment: image.alignment,
                    center_slice: image.center_slice,
                    repeat: image.repeat,
                    flip_horizontally: flip,
                    sampling: image.filter_quality.sampling(),
                };
                let mut painted = painted_image(paint, None);
                if image.opacity < 1.0 {
                    painted = Opacity::new(image.opacity, painted).into();
                }
                painted
            } else {
                decoration_image_widget(image)
            };
            if let Some(clip) = decoration.image_clip(Rect::from_origin_size(Offset::ZERO, size)) {
                painted = ClipPath::new(Arc::new(clip), painted).into();
            }
            layers.push(Positioned::fill(painted).into());
        }
        if let Some(border) = decoration.paint_border(size) {
            layers.push(CustomPaint::new(size, border).into());
        }
        // Shadows extend past the box, and Flutter's decoration painter
        // never clips them: no stack clip here or around the child.
        Stack::new(layers).clip_behavior(Clip::None).into()
    });
    // The decoration never takes a hit from the child it decorates.
    let layer: Widget = Positioned::fill(IgnorePointer::new(painted)).into();
    let children = match position {
        DecorationPosition::Background => [layer, child],
        DecorationPosition::Foreground => [child, layer],
    };
    let stack = Stack::new(children)
        .fit(StackFit::Passthrough)
        .clip_behavior(Clip::None);
    Widget::decorated_with_hit(
        None,
        None,
        None,
        CornerRadii::ZERO,
        stack.into(),
        Some(hit_decoration),
    )
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
    elliptical_rrect_path(rect, clockwise_radii(radius))
}

fn clockwise_radii(radius: BorderRadius) -> [Radius; 4] {
    [
        radius.top_left,
        radius.top_right,
        radius.bottom_right,
        radius.bottom_left,
    ]
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

/// Scale all corners together until adjacent radii fit on every side.
fn normalized_radii(size: Size, radii: [Radius; 4]) -> [(f32, f32); 4] {
    let (width, height) = (size.width.max(0.0), size.height.max(0.0));
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
    [tl, tr, br, bl].map(|(x, y)| (x * scale, y * scale))
}

/// A closed rounded rectangle with elliptical corners (top-left first,
/// clockwise), each corner a quarter conic. Radii that overflow a side are
/// scaled down together, as Skia's `SkRRect` does.
fn elliptical_rrect_path(rect: Rect, radii: [Radius; 4]) -> Path {
    let (left, top) = (rect.origin.x, rect.origin.y);
    let (width, height) = (rect.size.width.max(0.0), rect.size.height.max(0.0));
    let (right, bottom) = (left + width, top + height);
    let [tl, tr, br, bl] = normalized_radii(rect.size, radii);
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

/// The inputs of Flutter's `paintImage` (`painting/decoration_image.dart`)
/// that the plain image widget does not cover: a nine-slice center and a
/// horizontal flip.
#[derive(Clone, Debug)]
pub(crate) struct ImagePaint {
    pub(crate) image: incular_image::ImageHandle,
    pub(crate) scale: f32,
    pub(crate) fit: Option<crate::tree::ImageFit>,
    pub(crate) alignment: incular_config::Alignment,
    /// In logical image pixels (decoded pixels divided by `scale`).
    pub(crate) center_slice: Option<Rect>,
    pub(crate) repeat: crate::ImageRepeat,
    pub(crate) flip_horizontally: bool,
    pub(crate) sampling: incular_rendering::ImageSampling,
}

/// Flutter `applyBoxFit`: the source and destination sizes of a fit.
pub(crate) fn apply_box_fit(fit: crate::tree::ImageFit, input: Size, output: Size) -> (Size, Size) {
    use crate::tree::ImageFit;
    if input.width <= 0.0 || input.height <= 0.0 || output.width <= 0.0 || output.height <= 0.0 {
        return (Size::ZERO, Size::ZERO);
    }
    let wider = output.width / output.height > input.width / input.height;
    match fit {
        ImageFit::Fill => (input, output),
        ImageFit::Contain => {
            let destination = if wider {
                Size::new(input.width * output.height / input.height, output.height)
            } else {
                Size::new(output.width, input.height * output.width / input.width)
            };
            (input, destination)
        }
        ImageFit::Cover => {
            let source = if wider {
                Size::new(input.width, input.width * output.height / output.width)
            } else {
                Size::new(input.height * output.width / output.height, input.height)
            };
            (source, output)
        }
        ImageFit::FitWidth => {
            if wider {
                (
                    Size::new(input.width, input.width * output.height / output.width),
                    output,
                )
            } else {
                (
                    input,
                    Size::new(output.width, input.height * output.width / input.width),
                )
            }
        }
        ImageFit::FitHeight => {
            if wider {
                (
                    input,
                    Size::new(input.width * output.height / input.height, output.height),
                )
            } else {
                (
                    Size::new(input.height * output.width / output.height, input.height),
                    output,
                )
            }
        }
        ImageFit::None => {
            let source = Size::new(
                input.width.min(output.width),
                input.height.min(output.height),
            );
            (source, source)
        }
        ImageFit::ScaleDown => {
            let aspect = input.width / input.height;
            let mut destination = input;
            if destination.height > output.height {
                destination = Size::new(output.height * aspect, output.height);
            }
            if destination.width > output.width {
                destination = Size::new(output.width, output.width / aspect);
            }
            (input, destination)
        }
    }
}

/// Flutter `_generateImageTileRects`.
fn image_tile_rects(output: Rect, fundamental: Rect, repeat: crate::ImageRepeat) -> Vec<Rect> {
    use crate::ImageRepeat;
    let (stride_x, stride_y) = (fundamental.size.width, fundamental.size.height);
    let (mut start_x, mut stop_x, mut start_y, mut stop_y) = (0, 0, 0, 0);
    if stride_x > 0.0 && matches!(repeat, ImageRepeat::Repeat | ImageRepeat::RepeatX) {
        start_x = ((output.origin.x - fundamental.origin.x) / stride_x).floor() as i32;
        stop_x = ((output.origin.x + output.size.width
            - (fundamental.origin.x + fundamental.size.width))
            / stride_x)
            .ceil() as i32;
    }
    if stride_y > 0.0 && matches!(repeat, ImageRepeat::Repeat | ImageRepeat::RepeatY) {
        start_y = ((output.origin.y - fundamental.origin.y) / stride_y).floor() as i32;
        stop_y = ((output.origin.y + output.size.height
            - (fundamental.origin.y + fundamental.size.height))
            / stride_y)
            .ceil() as i32;
    }
    let count = (i64::from(stop_x) - i64::from(start_x) + 1).max(0)
        * (i64::from(stop_y) - i64::from(start_y) + 1).max(0);
    let mut tiles = Vec::with_capacity(count.min(16_384) as usize);
    for i in start_x..=stop_x {
        for j in start_y..=stop_y {
            if tiles.len() >= 16_384 {
                return tiles;
            }
            tiles.push(Rect::from_origin_size(
                Offset::new(
                    fundamental.origin.x + i as f32 * stride_x,
                    fundamental.origin.y + j as f32 * stride_y,
                ),
                fundamental.size,
            ));
        }
    }
    tiles
}

/// Skia `drawImageNine`: corners unscaled (one decoded pixel is `1 / scale`
/// logical pixels), edges stretched along one axis, the center along both.
/// Corners that do not fit shrink proportionally.
fn paint_nine(
    canvas: &mut Canvas,
    paint: &ImagePaint,
    center: Rect,
    destination: Rect,
    pixels: Size,
) {
    let scale = paint.scale.max(0.001);
    let split = |start: f32, end: f32, total: f32, origin: f32, extent: f32| {
        let (mut low, mut high) = (start / scale, (total - end) / scale);
        if low + high > extent && low + high > 0.0 {
            let k = extent / (low + high);
            low *= k;
            high *= k;
        }
        (
            [0.0, start, end, total],
            [
                origin,
                origin + low,
                origin + extent - high,
                origin + extent,
            ],
        )
    };
    let (sx, dx) = split(
        center.origin.x,
        center.origin.x + center.size.width,
        pixels.width,
        destination.origin.x,
        destination.size.width,
    );
    let (sy, dy) = split(
        center.origin.y,
        center.origin.y + center.size.height,
        pixels.height,
        destination.origin.y,
        destination.size.height,
    );
    for row in 0..3 {
        for column in 0..3 {
            let source = Rect::from_origin_size(
                Offset::new(sx[column], sy[row]),
                Size::new(sx[column + 1] - sx[column], sy[row + 1] - sy[row]),
            );
            let target = Rect::from_origin_size(
                Offset::new(dx[column], dy[row]),
                Size::new(dx[column + 1] - dx[column], dy[row + 1] - dy[row]),
            );
            if source.size.width > 0.0
                && source.size.height > 0.0
                && target.size.width > 0.0
                && target.size.height > 0.0
            {
                canvas.image_with_sampling(paint.image.clone(), source, target, paint.sampling);
            }
        }
    }
}

/// Flutter `paintImage` into `rect`, with nine-slicing and the horizontal
/// flip of `matchTextDirection`.
pub(crate) fn paint_image(paint: &ImagePaint, rect: Rect) -> DisplayList {
    let mut canvas = Canvas::default();
    if rect.size.width <= 0.0 || rect.size.height <= 0.0 {
        return canvas.finish();
    }
    let decoded = paint.image.decoded();
    let pixels = Size::new(decoded.width() as f32, decoded.height() as f32);
    let scale = paint.scale.max(0.001);
    let mut output = rect.size;
    let mut input = pixels;
    let border = paint.center_slice.map(|slice| {
        Size::new(
            pixels.width / scale - slice.size.width,
            pixels.height / scale - slice.size.height,
        )
    });
    if let Some(border) = border {
        output = Size::new(output.width - border.width, output.height - border.height);
        input = Size::new(
            input.width - border.width * scale,
            input.height - border.height * scale,
        );
    }
    let fit = paint.fit.unwrap_or(if paint.center_slice.is_none() {
        crate::tree::ImageFit::ScaleDown
    } else {
        crate::tree::ImageFit::Fill
    });
    let (source, destination) = apply_box_fit(
        fit,
        Size::new(input.width / scale, input.height / scale),
        output,
    );
    let source = Size::new(source.width * scale, source.height * scale);
    let mut destination = destination;
    if let Some(border) = border {
        output = Size::new(output.width + border.width, output.height + border.height);
        destination = Size::new(
            destination.width + border.width,
            destination.height + border.height,
        );
    }
    let mut repeat = paint.repeat;
    if repeat != crate::ImageRepeat::NoRepeat && destination == output {
        repeat = crate::ImageRepeat::NoRepeat;
    }
    let half_width = (output.width - destination.width) / 2.0;
    let half_height = (output.height - destination.height) / 2.0;
    let alignment_x = if paint.flip_horizontally {
        -paint.alignment.x
    } else {
        paint.alignment.x
    };
    let destination_rect = Rect::from_origin_size(
        Offset::new(
            rect.origin.x + half_width + alignment_x * half_width,
            rect.origin.y + half_height + paint.alignment.y * half_height,
        ),
        destination,
    );
    let mut saves = 0;
    if repeat != crate::ImageRepeat::NoRepeat {
        canvas.save_clip(rect);
        saves += 1;
    }
    if paint.flip_horizontally {
        let dx = rect.origin.x + rect.size.width / 2.0;
        canvas.save_transform(
            incular_core::Transform::translation(Offset::new(dx, 0.0))
                .then(incular_core::Transform::scale_non_uniform(-1.0, 1.0))
                .then(incular_core::Transform::translation(Offset::new(-dx, 0.0))),
        );
        saves += 1;
    }
    // `Alignment.inscribe(sourceSize, Offset.zero & inputSize)`.
    let half_x = (input.width - source.width) / 2.0;
    let half_y = (input.height - source.height) / 2.0;
    let source_rect = Rect::from_origin_size(
        Offset::new(
            half_x + paint.alignment.x * half_x,
            half_y + paint.alignment.y * half_y,
        ),
        source,
    );
    let center = paint.center_slice.map(|slice| {
        Rect::from_origin_size(
            Offset::new(slice.origin.x * scale, slice.origin.y * scale),
            Size::new(slice.size.width * scale, slice.size.height * scale),
        )
    });
    let mut paint_tile = |tile: Rect| match center {
        None => {
            canvas.image_with_sampling(paint.image.clone(), source_rect, tile, paint.sampling);
        }
        Some(center) => paint_nine(&mut canvas, paint, center, tile, pixels),
    };
    if repeat == crate::ImageRepeat::NoRepeat {
        paint_tile(destination_rect);
    } else {
        for tile in image_tile_rects(rect, destination_rect, repeat) {
            paint_tile(tile);
        }
    }
    for _ in 0..saves {
        canvas.restore();
    }
    canvas.finish()
}

/// An image widget painted by [`paint_image`] at the size of its box. The
/// image's own layout is kept by `sizing`, which lays out but paints
/// nothing.
pub(crate) fn painted_image(paint: ImagePaint, sizing: Option<Widget>) -> Widget {
    let painted: Widget = LayoutBuilder::new(move |_, constraints| {
        let size = constraints.biggest();
        CustomPaint::new(
            size,
            paint_image(&paint, Rect::from_origin_size(Offset::ZERO, size)),
        )
        .into()
    })
    .into();
    match sizing {
        None => painted,
        Some(sizing) => Stack::new([
            Widget::from(
                crate::Visibility::new(sizing)
                    .visible(false)
                    .maintain_state(true)
                    .maintain_size(true),
            ),
            Widget::from(Positioned::fill(painted)),
        ])
        .fit(StackFit::Passthrough)
        .clip_behavior(Clip::None)
        .into(),
    }
}

/// `BoxShadow.toPaint`'s `MaskFilter.blur(blurStyle, sigma)`. The
/// non-normal styles compose the shadow shape painted opaque the way Skia's
/// `SkBlurMask` combines the blurred mask `b` with the shape mask `s`:
/// solid is `s + b - s*b`, outer `b*(1 - s)`, inner `b*s`. The combined mask
/// takes the shadow color's alpha once, through an opacity group.
fn styled_shadow(decoration: &BoxDecoration, size: Size, shadow: &BoxShadow) -> Widget {
    use super::decoration::BlurStyle;
    let sigma = crate::utilities::banner_shadow_sigma(shadow.blur_radius);
    if shadow.blur_style == BlurStyle::Normal {
        return Widget::blur(
            sigma,
            CustomPaint::new(size, decoration.paint_shadow(size, shadow)).into(),
        );
    }
    let opaque = BoxShadow {
        color: Color {
            alpha: 255,
            ..shadow.color
        },
        ..*shadow
    };
    let shape: Widget = CustomPaint::new(size, decoration.paint_shadow(size, &opaque)).into();
    let blurred = Widget::blur(sigma, shape.clone());
    let mask = match shadow.blur_style {
        BlurStyle::Outer => Widget::blend(incular_rendering::BlendMode::DstOut, shape.clone()),
        BlurStyle::Inner => Widget::blend(incular_rendering::BlendMode::DstIn, shape.clone()),
        BlurStyle::Solid | BlurStyle::Normal => shape,
    };
    let group: Widget = Stack::new([blurred, mask]).clip_behavior(Clip::None).into();
    Opacity::new(f32::from(shadow.color.alpha) / 255.0, group).into()
}
