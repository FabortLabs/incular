//! Layering and value interpolation of physical and directional box borders.

use std::{fmt, ops::Add};

use incular_config::{EdgeInsets, EdgeInsetsGeometry, TextDirection};
use incular_core::{Color, Lerp, Rect};
use incular_rendering::{Canvas, Paint, Path};

use super::{
    Border, BorderComposition, BorderDirectional, BorderSide, BorderStyle, BoxShape, ShapeBorder,
    shape_borders::deflate_border,
};

/// One physical or directional layer in a compound box outline.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BoxBorderLayer {
    Physical(Border),
    Directional(BorderDirectional),
}

impl BoxBorderLayer {
    fn combine(self, other: Self) -> Option<Self> {
        match (self, other) {
            (Self::Physical(a), Self::Physical(b)) => match a + b {
                BorderComposition::Merged(value) => Some(Self::Physical(value)),
                BorderComposition::Layered { .. } => None,
            },
            (Self::Directional(a), Self::Directional(b))
                if a.top.can_merge(&b.top)
                    && a.start.can_merge(&b.start)
                    && a.end.can_merge(&b.end)
                    && a.bottom.can_merge(&b.bottom) =>
            {
                Some(Self::Directional(a.merge(b)))
            }
            (Self::Physical(physical), Self::Directional(logical))
            | (Self::Directional(logical), Self::Physical(physical)) => {
                if !physical.top.can_merge(&logical.top)
                    || !physical.bottom.can_merge(&logical.bottom)
                {
                    return None;
                }
                let top = physical.top.merge(&logical.top);
                let bottom = physical.bottom.merge(&logical.bottom);
                if logical.start == BorderSide::NONE && logical.end == BorderSide::NONE {
                    Some(Self::Physical(Border {
                        top,
                        bottom,
                        ..physical
                    }))
                } else if physical.left == BorderSide::NONE && physical.right == BorderSide::NONE {
                    Some(Self::Directional(BorderDirectional {
                        top,
                        bottom,
                        ..logical
                    }))
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    fn interpolate(self, other: Self, t: f32) -> Option<Self> {
        match (self, other) {
            (Self::Physical(a), Self::Physical(b)) => Some(Self::Physical(a.lerp(&b, t))),
            (Self::Directional(a), Self::Directional(b)) => Some(Self::Directional(a.lerp(&b, t))),
            _ => None,
        }
    }

    fn scale(self, t: f32) -> Self {
        match self {
            Self::Physical(b) => Self::Physical(b.scale(t)),
            Self::Directional(b) => Self::Directional(b.scale(t)),
        }
    }

    fn dimensions(self) -> EdgeInsetsGeometry {
        match self {
            Self::Physical(b) => b.dimensions().into(),
            Self::Directional(b) => b.dimensions().into(),
        }
    }

    fn resolve(self, direction: TextDirection) -> Border {
        match self {
            Self::Physical(b) => b,
            Self::Directional(b) => b.resolve(direction),
        }
    }
}

/// Compatible adjacent borders merge; incompatible ones retain their
/// independent paint, dimensions, and interpolation. In `a + b`,
/// `b` is on the outside and paints first.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BoxBorderStack {
    layers: Vec<BoxBorderLayer>,
}

impl BoxBorderStack {
    /// Retained layers in painting order, outside first.
    #[must_use]
    pub fn layers(&self) -> &[BoxBorderLayer] {
        &self.layers
    }

    /// Border insets without prematurely resolving directional sides.
    #[must_use]
    pub fn dimensions_geometry(&self) -> EdgeInsetsGeometry {
        self.layers
            .iter()
            .copied()
            .map(BoxBorderLayer::dimensions)
            .reduce(Add::add)
            .unwrap_or(EdgeInsetsGeometry::ZERO)
    }

    #[must_use]
    pub fn scale(&self, t: f32) -> Self {
        Self {
            layers: self.layers.iter().map(|layer| layer.scale(t)).collect(),
        }
    }
}

impl From<Border> for BoxBorderStack {
    fn from(value: Border) -> Self {
        Self {
            layers: vec![BoxBorderLayer::Physical(value)],
        }
    }
}

impl From<BorderDirectional> for BoxBorderStack {
    fn from(value: BorderDirectional) -> Self {
        Self {
            layers: vec![BoxBorderLayer::Directional(value)],
        }
    }
}

impl From<BorderComposition> for BoxBorderStack {
    fn from(value: BorderComposition) -> Self {
        match value {
            BorderComposition::Merged(border) => border.into(),
            BorderComposition::Layered { outside, inside } => Self {
                layers: vec![
                    BoxBorderLayer::Physical(outside),
                    BoxBorderLayer::Physical(inside),
                ],
            },
        }
    }
}

impl<Rhs: Into<BoxBorderStack>> Add<Rhs> for BoxBorderStack {
    type Output = Self;

    fn add(self, rhs: Rhs) -> Self {
        let mut outside = rhs.into();
        let mut inside = self.layers;
        let merge_boundary = outside.layers.len() == 1 || inside.len() == 1;
        if merge_boundary
            && let (Some(outer), Some(inner)) = (outside.layers.last_mut(), inside.first())
            && let Some(merged) = inner.combine(*outer)
        {
            *outer = merged;
            inside.remove(0);
        }
        outside.layers.extend(inside);
        outside
    }
}

impl Add<Border> for BorderComposition {
    type Output = BoxBorderStack;
    fn add(self, rhs: Border) -> BoxBorderStack {
        BoxBorderStack::from(self) + rhs
    }
}

impl Add<BorderComposition> for Border {
    type Output = BoxBorderStack;
    fn add(self, rhs: BorderComposition) -> BoxBorderStack {
        BoxBorderStack::from(self) + rhs
    }
}

impl Add<BoxBorderStack> for Border {
    type Output = BoxBorderStack;
    fn add(self, rhs: BoxBorderStack) -> BoxBorderStack {
        BoxBorderStack::from(self) + rhs
    }
}

impl Add<BorderDirectional> for Border {
    type Output = BoxBorderStack;
    fn add(self, rhs: BorderDirectional) -> BoxBorderStack {
        BoxBorderStack::from(self) + rhs
    }
}

impl Add<Border> for BorderDirectional {
    type Output = BoxBorderStack;
    fn add(self, rhs: Border) -> BoxBorderStack {
        BoxBorderStack::from(self) + rhs
    }
}

impl Add for BorderDirectional {
    type Output = BoxBorderStack;
    fn add(self, rhs: Self) -> BoxBorderStack {
        BoxBorderStack::from(self) + rhs
    }
}

impl Add<BoxBorderStack> for BorderDirectional {
    type Output = BoxBorderStack;
    fn add(self, rhs: BoxBorderStack) -> BoxBorderStack {
        BoxBorderStack::from(self) + rhs
    }
}

impl Lerp for BoxBorderStack {
    fn lerp(&self, other: &Self, t: f32) -> Self {
        if t == 0. {
            return self.clone();
        }
        if t == 1. {
            return other.clone();
        }
        let count = self.layers.len().max(other.layers.len());
        let mut layers = Vec::with_capacity(count);
        for index in 0..count {
            let a = self.layers.get(index).copied();
            let b = other.layers.get(index).copied();
            if let (Some(a), Some(b)) = (a, b)
                && let Some(value) = a.interpolate(b, t)
            {
                layers.push(value);
                continue;
            }
            // Incoming incompatible shapes paint outside the outgoing one.
            layers.extend(b.map(|b| b.scale(t)));
            layers.extend(a.map(|a| a.scale(1. - t)));
        }
        Self { layers }
    }
}

impl ShapeBorder for BoxBorderStack {
    fn dimensions(&self, direction: Option<TextDirection>) -> EdgeInsets {
        self.dimensions_geometry()
            .resolve(direction.unwrap_or(TextDirection::Ltr))
    }

    fn outer_path(&self, rect: Rect) -> Path {
        Border::default().outer_path(rect)
    }

    fn inner_path(&self, rect: Rect) -> Path {
        self.inner_path_with_direction(rect, None)
    }

    fn inner_path_with_direction(&self, mut rect: Rect, direction: Option<TextDirection>) -> Path {
        let direction = direction.unwrap_or(TextDirection::Ltr);
        let Some((inner, outer)) = self.layers.split_last() else {
            return self.outer_path(rect);
        };
        for side in outer {
            rect = deflate_border(rect, side.dimensions().resolve(direction));
        }
        inner.resolve(direction).inner_path(rect)
    }

    fn prefer_paint_interior(&self) -> bool {
        true
    }

    fn paint_interior(
        &self,
        canvas: &mut Canvas,
        rect: Rect,
        paint: &Paint,
        direction: Option<TextDirection>,
    ) {
        Border::default().paint_interior(canvas, rect, paint, direction);
    }

    fn paint(&self, canvas: &mut Canvas, rect: Rect) {
        self.paint_with_direction(canvas, rect, None);
    }

    fn paint_with_direction(
        &self,
        canvas: &mut Canvas,
        mut rect: Rect,
        direction: Option<TextDirection>,
    ) {
        for &border in &self.layers {
            match border {
                BoxBorderLayer::Physical(b) => {
                    ShapeBorder::paint(&b, canvas, rect);
                }
                BoxBorderLayer::Directional(b) => {
                    let _ = b.paint(canvas, rect, direction, BoxShape::Rectangle, None);
                }
            }
            rect = deflate_border(
                rect,
                border
                    .dimensions()
                    .resolve(direction.unwrap_or(TextDirection::Ltr)),
            );
        }
    }
}

impl fmt::Display for BorderSide {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let side = *self;
        f.write_str("BorderSide(")?;
        let mut separated = false;
        let scalar = |value: f32| {
            let mut text = value.to_string();
            if value.is_finite() && !text.contains('.') && !text.contains('e') {
                text.push_str(".0");
            }
            text
        };
        if side.color != Color::BLACK {
            let c = side.color;
            let fraction = |channel| f64::from(channel) / 255.;
            write!(
                f,
                "color: Color(alpha: {:.4}, red: {:.4}, green: {:.4}, blue: {:.4}, colorSpace: ColorSpace.sRGB)",
                fraction(c.alpha),
                fraction(c.red),
                fraction(c.green),
                fraction(c.blue)
            )?;
            separated = true;
        }
        for argument in [
            (side.width != 1.).then(|| format!("width: {}", scalar(side.width))),
            (side.stroke_align != -1.)
                .then(|| format!("strokeAlign: {}", scalar(side.stroke_align))),
            (side.style == BorderStyle::None).then(|| String::from("style: none")),
        ]
        .into_iter()
        .flatten()
        {
            if separated {
                f.write_str(", ")?;
            }
            f.write_str(&argument)?;
            separated = true;
        }
        f.write_str(")")
    }
}

fn format_edges(
    f: &mut fmt::Formatter<'_>,
    kind: &str,
    sides: [(&str, BorderSide); 4],
) -> fmt::Result {
    write!(f, "{kind}(")?;
    for (index, (edge, side)) in sides
        .into_iter()
        .filter(|(_, side)| *side != BorderSide::NONE)
        .enumerate()
    {
        if index != 0 {
            f.write_str(", ")?;
        }
        write!(f, "{edge}: {side}")?;
    }
    f.write_str(")")
}

impl fmt::Display for Border {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_uniform() {
            return write!(f, "Border.all({})", self.top);
        }
        format_edges(
            f,
            "Border",
            [
                ("top", self.top),
                ("right", self.right),
                ("bottom", self.bottom),
                ("left", self.left),
            ],
        )
    }
}

impl fmt::Display for BorderDirectional {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        format_edges(
            f,
            "BorderDirectional",
            [
                ("top", self.top),
                ("start", self.start),
                ("end", self.end),
                ("bottom", self.bottom),
            ],
        )
    }
}

impl fmt::Display for BoxBorderStack {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, layer) in self.layers.iter().rev().enumerate() {
            if index != 0 {
                f.write_str(" + ")?;
            }
            match layer {
                BoxBorderLayer::Physical(border) => border.fmt(f)?,
                BoxBorderLayer::Directional(border) => border.fmt(f)?,
            }
        }
        Ok(())
    }
}
