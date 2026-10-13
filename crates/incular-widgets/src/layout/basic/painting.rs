//! Painting and placeholder layout primitives.

use std::sync::Arc;

use incular_core::{Color, Offset, Size};
use incular_rendering::{Canvas, DisplayList, Path, Stroke};
use typed_builder::TypedBuilder;

use super::SizedBox;
use crate::{DecoratedBox, IgnorePointer, Widget, WidgetKind, macros::widget_from_kind};

/// Paints a solid background color behind a child widget.
#[derive(Clone, Debug, PartialEq, TypedBuilder)]
pub struct ColoredBox {
    #[builder(setter(into))]
    color: Color,
    #[builder(default, setter(strip_option, into))]
    pub(super) child: Option<Widget>,
}

impl ColoredBox {
    /// Creates a ColoredBox wrapping a child with a background color.
    #[must_use]
    pub fn new(color: Color, child: impl Into<Widget>) -> Self {
        Self {
            color,
            child: Some(child.into()),
        }
    }

    /// Sets the background color.
    #[must_use]
    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    /// Sets the child widget.
    #[must_use]
    pub fn child(mut self, child: impl Into<Widget>) -> Self {
        self.child = Some(child.into());
        self
    }
}

impl From<ColoredBox> for Widget {
    fn from(value: ColoredBox) -> Self {
        let child = value.child.unwrap_or_else(|| SizedBox::shrink().into());
        DecoratedBox::new(child).background(value.color).into()
    }
}

/// Isolates display list painting and caching into a dedicated layer.
#[derive(Clone, Debug, PartialEq, TypedBuilder)]
pub struct RepaintBoundary {
    #[builder(setter(into))]
    child: Widget,
}

impl RepaintBoundary {
    /// Creates a RepaintBoundary.
    #[must_use]
    pub fn new(child: impl Into<Widget>) -> Self {
        Self {
            child: child.into(),
        }
    }
}

widget_from_kind!(RepaintBoundary => RepaintBoundary { child });

/// Paints an explicit [`DisplayList`] with a fixed size.
#[derive(Clone, Debug, PartialEq, TypedBuilder)]
pub struct CustomPaint {
    #[builder(default = Size::ZERO)]
    size: Size,
    display_list: DisplayList,
}

impl CustomPaint {
    /// Creates a CustomPaint widget.
    #[must_use]
    pub fn new(size: Size, display_list: DisplayList) -> Self {
        Self { size, display_list }
    }

    /// Creates a CustomPaint widget from a custom painter.
    #[must_use]
    pub fn from_painter(size: Size, painter: impl crate::CustomPainter) -> Self {
        let display_list = painter.paint(size);
        Self { size, display_list }
    }
}

widget_from_kind!(CustomPaint => CustomPaint { size, display_list });

/// A placeholder box with cross lines and fallback sizing.
#[derive(Clone, Debug, PartialEq, TypedBuilder)]
pub struct Placeholder {
    #[builder(default = Color::rgba(69, 90, 100, 255))]
    color: Color,
    #[builder(default = 2.0, setter(transform = |width: f32| width.max(0.1)))]
    stroke_width: f32,
    #[builder(default = 400.0, setter(transform = |width: f32| width.max(0.0)))]
    fallback_width: f32,
    #[builder(default = 400.0, setter(transform = |height: f32| height.max(0.0)))]
    fallback_height: f32,
}

impl Default for Placeholder {
    fn default() -> Self {
        Self::new()
    }
}

impl Placeholder {
    #[must_use]
    pub fn new() -> Self {
        Self {
            color: Color::rgba(69, 90, 100, 255),
            stroke_width: 2.0,
            fallback_width: 400.0,
            fallback_height: 400.0,
        }
    }

    #[must_use]
    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    #[must_use]
    pub fn stroke_width(mut self, width: f32) -> Self {
        self.stroke_width = width.max(0.1);
        self
    }

    #[must_use]
    pub fn fallback_width(mut self, width: f32) -> Self {
        self.fallback_width = width.max(0.0);
        self
    }

    #[must_use]
    pub fn fallback_height(mut self, height: f32) -> Self {
        self.fallback_height = height.max(0.0);
        self
    }
}

/// Strokes the outline rectangle and both diagonals, as Flutter's
/// `_PlaceholderPainter` does, with the placeholder's stroke width.
#[derive(Clone, Copy, Debug, PartialEq)]
struct PlaceholderPainter {
    color: Color,
    stroke_width: f32,
}

impl crate::CustomPainter for PlaceholderPainter {
    fn paint(&self, size: Size) -> DisplayList {
        let top_left = Offset::ZERO;
        let top_right = Offset::new(size.width, 0.0);
        let bottom_right = Offset::new(size.width, size.height);
        let bottom_left = Offset::new(0.0, size.height);
        let mut lines = Path::builder();
        lines
            .move_to(top_left)
            .line_to(top_right)
            .line_to(bottom_right)
            .line_to(bottom_left)
            .close();
        lines.move_to(top_right).line_to(bottom_left);
        lines.move_to(top_left).line_to(bottom_right);
        let mut canvas = Canvas::default();
        canvas.stroke_path(
            Arc::new(lines.build()),
            self.color,
            Stroke {
                width: self.stroke_width,
                ..Stroke::default()
            },
        );
        canvas.finish()
    }

    fn should_repaint(&self, old: &Self) -> bool {
        self != old
    }
}

impl From<Placeholder> for Widget {
    fn from(value: Placeholder) -> Self {
        // Flutter's placeholder painter reports `hitTest` as false, so the
        // outline never absorbs pointer events.
        let painter = PlaceholderPainter {
            color: value.color,
            stroke_width: value.stroke_width,
        };
        IgnorePointer::new(
            SizedBox::new()
                .width(value.fallback_width)
                .height(value.fallback_height)
                .child(CustomPaint::from_painter(
                    Size::new(value.fallback_width, value.fallback_height),
                    painter,
                )),
        )
        .into()
    }
}
