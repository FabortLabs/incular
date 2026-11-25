//! Physical outer sides and internal dividers for a table grid.

use std::sync::Arc;

use incular_config::EdgeInsets;
use incular_core::{Lerp, Offset, Rect};
use incular_rendering::{Canvas, FillRule, Path};

use super::box_painter::{deflate_rrect, fill_between, inflate_rrect, paint_border_sides};
use super::{Border, BorderRadius, BorderSide, BorderStyle};

/// Optional table strokes. By default all six sides are absent.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TableBorder {
    pub top: BorderSide,
    pub right: BorderSide,
    pub bottom: BorderSide,
    pub left: BorderSide,
    pub horizontal_inside: BorderSide,
    pub vertical_inside: BorderSide,
    pub border_radius: BorderRadius,
}

impl Default for TableBorder {
    fn default() -> Self {
        Self::all(BorderSide::NONE)
    }
}

impl TableBorder {
    /// Uses one side for all outer edges and inner dividers.
    #[must_use]
    pub const fn all(side: BorderSide) -> Self {
        Self::symmetric(side, side)
    }

    /// Uses separate sides for inner dividers and outer edges.
    #[must_use]
    pub const fn symmetric(inside: BorderSide, outside: BorderSide) -> Self {
        Self {
            top: outside,
            right: outside,
            bottom: outside,
            left: outside,
            horizontal_inside: inside,
            vertical_inside: inside,
            border_radius: BorderRadius::ZERO,
        }
    }

    #[must_use]
    pub const fn border_radius(mut self, radius: BorderRadius) -> Self {
        self.border_radius = radius;
        self
    }

    /// Widths of the four physical edges, excluding dividers.
    #[must_use]
    pub fn dimensions(self) -> EdgeInsets {
        EdgeInsets::only(
            self.left.width,
            self.top.width,
            self.right.width,
            self.bottom.width,
        )
    }

    #[must_use]
    pub fn is_uniform(self) -> bool {
        self.sides().iter().all(|side| sides_match(self.top, *side))
    }

    pub(crate) fn has_paint(self) -> bool {
        self.sides()
            .iter()
            .any(|side| side.style == BorderStyle::Solid && side.color.alpha != 0)
    }

    /// Scales each side. Like Flutter, the returned radius is zero.
    #[must_use]
    pub fn scale(self, factor: f32) -> Self {
        Self {
            top: self.top.scale(factor),
            right: self.right.scale(factor),
            bottom: self.bottom.scale(factor),
            left: self.left.scale(factor),
            horizontal_inside: self.horizontal_inside.scale(factor),
            vertical_inside: self.vertical_inside.scale(factor),
            border_radius: BorderRadius::ZERO,
        }
    }

    /// Interpolates all six sides, treating an absent endpoint as no border.
    #[must_use]
    pub fn lerp(a: Option<Self>, b: Option<Self>, t: f32) -> Option<Self> {
        match (a, b) {
            (a, b) if a == b => a,
            (None, b) => b.map(|b| b.scale(t)),
            (a, None) => a.map(|a| a.scale(1. - t)),
            (Some(a), Some(b)) => Some(Self {
                top: a.top.lerp(&b.top, t),
                right: a.right.lerp(&b.right, t),
                bottom: a.bottom.lerp(&b.bottom, t),
                left: a.left.lerp(&b.left, t),
                horizontal_inside: a.horizontal_inside.lerp(&b.horizontal_inside, t),
                vertical_inside: a.vertical_inside.lerp(&b.vertical_inside, t),
                border_radius: BorderRadius::ZERO,
            }),
        }
    }

    /// Paints vertical dividers, horizontal dividers, and the outer border,
    /// in that order. Divider positions are relative to the rectangle.
    pub fn paint(self, canvas: &mut Canvas, rect: Rect, rows: &[f32], columns: &[f32]) {
        if !self.has_paint() {
            return;
        }
        paint_dividers(canvas, self.vertical_inside, rect, columns, true);
        paint_dividers(canvas, self.horizontal_inside, rect, rows, false);
        self.paint_outer(canvas, rect);
    }

    fn sides(self) -> [BorderSide; 6] {
        [
            self.top,
            self.right,
            self.bottom,
            self.left,
            self.horizontal_inside,
            self.vertical_inside,
        ]
    }

    fn paint_outer(self, canvas: &mut Canvas, rect: Rect) {
        let outside = Border::only(self.top, self.right, self.bottom, self.left);
        let [top, right, bottom, left] = [self.top, self.right, self.bottom, self.left];
        let sides = [top, right, bottom, left];
        let mut visible = sides
            .into_iter()
            .filter(|side| side.style == BorderStyle::Solid);
        let Some(first) = visible.next() else { return };
        if self.border_radius != BorderRadius::ZERO {
            if sides.iter().all(|side| sides_match(top, *side)) {
                if top.width > 0. {
                    fill_between(
                        canvas,
                        inflate_rrect(rect, self.border_radius, EdgeInsets::ZERO),
                        deflate_rrect(rect, self.border_radius, EdgeInsets::all(top.width)),
                        top.color,
                    );
                }
                return;
            }
            if visible.all(|side| side.color == first.color) {
                let [top, right, bottom, left] = sides.map(|side| {
                    if side.style == BorderStyle::None {
                        BorderSide::NONE
                    } else {
                        side
                    }
                });
                let inner = EdgeInsets::only(
                    left.stroke_inset(),
                    top.stroke_inset(),
                    right.stroke_inset(),
                    bottom.stroke_inset(),
                );
                let outer = EdgeInsets::only(
                    left.stroke_outset(),
                    top.stroke_outset(),
                    right.stroke_outset(),
                    bottom.stroke_outset(),
                );
                fill_between(
                    canvas,
                    inflate_rrect(rect, self.border_radius, outer),
                    deflate_rrect(rect, self.border_radius, inner),
                    first.color,
                );
                return;
            }
        }
        paint_border_sides(canvas, rect, outside);
    }
}

fn sides_match(a: BorderSide, b: BorderSide) -> bool {
    a.color == b.color && a.width == b.width && a.style == b.style
}

fn paint_dividers(
    canvas: &mut Canvas,
    side: BorderSide,
    rect: Rect,
    positions: &[f32],
    vertical: bool,
) {
    if side.style == BorderStyle::None || side.color.alpha == 0 || positions.is_empty() {
        return;
    }
    let mut path = Path::builder();
    let from = rect.origin;
    for &offset in positions {
        let (start, end) = if vertical {
            (
                Offset::new(from.x + offset, from.y),
                Offset::new(from.x + offset, from.y + rect.size.height),
            )
        } else {
            (
                Offset::new(from.x, from.y + offset),
                Offset::new(from.x + rect.size.width, from.y + offset),
            )
        };
        path.move_to(start).line_to(end);
    }
    canvas.draw_path(Arc::new(path.build()), &side.to_paint(), FillRule::NonZero);
}
