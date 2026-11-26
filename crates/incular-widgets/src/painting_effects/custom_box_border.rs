//! The extension contract for custom borders in a box decoration.

use std::{any::Any, fmt::Debug, rc::Rc, sync::Arc};

use incular_config::{EdgeInsets, TextDirection};
use incular_core::{Offset, Rect, Size};
use incular_rendering::{Canvas, Path};

use super::decoration::{Border, BorderDirectional, BorderSide, BoxShape};
use super::geometry::BorderRadius;
use super::shape_borders::ShapeBorder;

/// One present endpoint passed to a custom box border's interpolation hook.
#[derive(Clone, Copy, Debug)]
pub enum BoxBorderValue<'a> {
    Physical(Border),
    Directional(BorderDirectional),
    Custom(&'a dyn CustomBoxBorder),
}

impl BoxBorderValue<'_> {
    #[must_use]
    pub fn downcast_ref<T: CustomBoxBorder>(&self) -> Option<&T> {
        let Self::Custom(value) = self else {
            return None;
        };
        let value: &dyn Any = *value;
        value.downcast_ref()
    }
}

/// Flutter's custom BoxBorder contract for BoxDecoration. Use the
/// direction only when required by logical padding or painting.
pub trait CustomBoxBorder: Any + Debug + Send + Sync {
    fn top(&self) -> BorderSide;

    fn bottom(&self) -> BorderSide;

    fn is_uniform(&self) -> bool;

    fn dimensions(&self, direction: Option<TextDirection>) -> EdgeInsets;

    fn needs_text_direction(&self) -> bool {
        false
    }

    /// Draws the stroke into the ordinary renderer-neutral canvas. Returns
    /// false when this border produced no paint.
    fn paint(
        &self,
        canvas: &mut Canvas,
        rect: Rect,
        direction: Option<TextDirection>,
        shape: BoxShape,
        radius: Option<BorderRadius>,
    ) -> bool;

    fn scale(&self, factor: f32) -> CustomBoxBorderHandle;

    fn lerp_from(
        &self,
        other: Option<BoxBorderValue<'_>>,
        t: f32,
    ) -> Option<CustomBoxBorderHandle> {
        other.is_none().then(|| self.scale(t))
    }

    fn lerp_to(&self, other: Option<BoxBorderValue<'_>>, t: f32) -> Option<CustomBoxBorderHandle> {
        other.is_none().then(|| self.scale(1. - t))
    }

    /// Optional value equality for immutable implementations. Leave this
    /// false when only a shared instance is equal.
    fn same_value(&self, _other: &dyn CustomBoxBorder) -> bool {
        false
    }
}

/// A shared custom border. Equality is by instance unless both custom
/// implementations explicitly agree that their values are equal.
#[derive(Clone, Debug)]
pub struct CustomBoxBorderHandle {
    border: Arc<dyn CustomBoxBorder>,
    direction: Option<TextDirection>,
}

impl PartialEq for CustomBoxBorderHandle {
    fn eq(&self, other: &Self) -> bool {
        self.direction == other.direction
            && (self.same_instance(other)
                || (self.border.same_value(other.behavior())
                    && other.border.same_value(self.behavior())))
    }
}

impl CustomBoxBorderHandle {
    #[must_use]
    pub fn new(border: impl CustomBoxBorder) -> Self {
        Self {
            border: Arc::new(border),
            direction: None,
        }
    }

    #[must_use]
    pub fn behavior(&self) -> &dyn CustomBoxBorder {
        self.border.as_ref()
    }

    #[must_use]
    pub fn downcast_ref<T: CustomBoxBorder>(&self) -> Option<&T> {
        let border: &dyn Any = self.border.as_ref();
        border.downcast_ref()
    }

    pub(super) fn same_instance(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.border, &other.border)
    }

    pub(super) fn resolved_for_direction(mut self, direction: TextDirection) -> Self {
        if self.border.needs_text_direction() {
            self.direction = Some(direction);
        }
        self
    }

    pub(super) fn needs_resolution(&self) -> bool {
        self.direction.is_none() && self.border.needs_text_direction()
    }

    pub(super) fn dimensions(&self, direction: Option<TextDirection>) -> EdgeInsets {
        self.border.dimensions(direction.or(self.direction))
    }

    pub(super) fn paint(
        &self,
        canvas: &mut Canvas,
        rect: Rect,
        direction: Option<TextDirection>,
        shape: BoxShape,
        radius: Option<BorderRadius>,
    ) -> bool {
        self.border
            .paint(canvas, rect, direction.or(self.direction), shape, radius)
    }

    pub(super) fn scale(&self, factor: f32) -> Self {
        let mut result = self.border.scale(factor);
        result.direction = self.direction;
        result
    }
}

// A square ShapeDecoration takes the BoxBorder itself, not a synthesized
// uniform border. Rounded and circular conversions use its top side.
impl ShapeBorder for CustomBoxBorderHandle {
    fn dimensions(&self, direction: Option<TextDirection>) -> EdgeInsets {
        Self::dimensions(self, direction)
    }

    fn outer_path(&self, rect: Rect) -> Path {
        Border::default().outer_path(rect)
    }

    fn inner_path(&self, rect: Rect) -> Path {
        self.inner_path_with_direction(rect, None)
    }

    fn inner_path_with_direction(&self, rect: Rect, direction: Option<TextDirection>) -> Path {
        let inset = self.dimensions(direction);
        let inner = Rect::from_origin_size(
            rect.origin + Offset::new(inset.left, inset.top),
            Size::new(
                (rect.size.width - inset.left - inset.right).max(0.),
                (rect.size.height - inset.top - inset.bottom).max(0.),
            ),
        );
        Border::default().outer_path(inner)
    }

    fn needs_text_direction(&self) -> bool {
        self.needs_resolution()
    }

    fn paint(&self, canvas: &mut Canvas, rect: Rect) {
        self.paint_with_direction(canvas, rect, None);
    }

    fn paint_with_direction(
        &self,
        canvas: &mut Canvas,
        rect: Rect,
        direction: Option<TextDirection>,
    ) {
        Self::paint(self, canvas, rect, direction, BoxShape::Rectangle, None);
    }

    fn scaled(&self, factor: f32) -> Option<Rc<dyn ShapeBorder>> {
        Some(Rc::new(self.scale(factor)))
    }
}
