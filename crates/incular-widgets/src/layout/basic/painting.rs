//! Painting and placeholder layout primitives.

use std::{cell::{Cell, RefCell}, rc::Rc, sync::Arc};

use incular_config::{Clip, StackFit};
use incular_core::{Color, Offset, Size};
use incular_rendering::{Canvas, DisplayList, Path, Stroke};
use typed_builder::TypedBuilder;

use super::SizedBox;
use crate::painting_effects::PainterDelegate;
use crate::{LimitedBox, Positioned, Stack, Widget, WidgetKind, macros::widget_from_kind};

/// Paints a solid background color behind a child widget.
#[derive(Clone, Debug, PartialEq, TypedBuilder)]
pub struct ColoredBox {
    #[builder(setter(into))]
    color: Color,
    #[builder(default = true)]
    is_anti_alias: bool,
    #[builder(default, setter(strip_option, into))]
    pub(super) child: Option<Widget>,
}

impl ColoredBox {
    /// Creates a ColoredBox wrapping a child with a background color.
    #[must_use]
    pub fn new(color: Color, child: impl Into<Widget>) -> Self {
        Self::empty(color).child(child)
    }

    /// Creates a colored box without a child. Its size is the incoming minimum.
    #[must_use]
    pub fn empty(color: Color) -> Self {
        Self {
            color,
            is_anti_alias: true,
            child: None,
        }
    }

    /// Uses fractional edge coverage, including under affine transforms.
    /// Enabled by default; disable to avoid seams between adjoining boxes.
    #[must_use]
    pub fn is_anti_alias(mut self, enabled: bool) -> Self {
        self.is_anti_alias = enabled;
        self
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

widget_from_kind!(ColoredBox => Colored { color, is_anti_alias, child });

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

/// Paints a custom painter or an explicit [`DisplayList`].
///
/// As Flutter's `CustomPaint`, the painter paints before the child and the
/// foreground painter after it. Without a child the preferred size applies
/// (or the constraints' biggest size after [`Self::fill_to_constraints`]);
/// with a child the size is the child's. Unless overridden by the painter,
/// the foreground passes hits through and the background accepts them.
#[derive(Clone, Debug, PartialEq, TypedBuilder)]
pub struct CustomPaint {
    #[builder(default = Some(Size::ZERO))]
    size: Option<Size>,
    display_list: DisplayList,
    #[builder(default, setter(strip_option))]
    painter: Option<PainterDelegate>,
    #[builder(default, setter(strip_option))]
    foreground_painter: Option<PainterDelegate>,
    #[builder(default, setter(strip_option, into))]
    child: Option<Widget>,
}

impl CustomPaint {
    /// Creates a CustomPaint widget.
    #[must_use]
    pub fn new(size: Size, display_list: DisplayList) -> Self {
        Self {
            size: Some(size),
            display_list,
            painter: None,
            foreground_painter: None,
            child: None,
        }
    }

    /// Creates a CustomPaint widget from a custom painter at a fixed size.
    /// The painter runs at paint time with the laid-out size.
    #[must_use]
    pub fn from_painter(size: Size, painter: impl crate::CustomPainter + 'static) -> Self {
        Self::new(size, DisplayList::default()).painter(painter)
    }

    /// Creates a CustomPaint widget from a painter that fills the constraints,
    /// Flutter's `Size.infinite` preferred size.
    #[must_use]
    pub fn from_painter_filling(painter: impl crate::CustomPainter + 'static) -> Self {
        Self::from_painter(Size::ZERO, painter).fill_to_constraints()
    }

    /// Sets the painter that paints behind the child.
    #[must_use]
    pub fn painter(mut self, painter: impl crate::CustomPainter + 'static) -> Self {
        self.painter = Some(PainterDelegate::new(painter));
        self
    }

    /// Sets the painter that paints over the child.
    #[must_use]
    pub fn foreground_painter(mut self, painter: impl crate::CustomPainter + 'static) -> Self {
        self.foreground_painter = Some(PainterDelegate::new(painter));
        self
    }

    /// Sets the child widget, which determines the size.
    #[must_use]
    pub fn child(mut self, child: impl Into<Widget>) -> Self {
        self.child = Some(child.into());
        self
    }

    /// Sets the preferred size used when there is no child.
    #[must_use]
    pub fn preferred_size(mut self, size: Size) -> Self {
        self.size = Some(size);
        self
    }

    /// Fills the constraints when there is no child (Flutter `Size.infinite`).
    #[must_use]
    pub fn fill_to_constraints(mut self) -> Self {
        self.size = None;
        self
    }
}

impl From<CustomPaint> for Widget {
    fn from(value: CustomPaint) -> Self {
        let CustomPaint {
            size,
            display_list,
            painter,
            foreground_painter,
            child,
        } = value;
        let background_semantics = painter
            .as_ref()
            .filter(|painter| painter.semantics_builder().is_some())
            .cloned();
        let foreground_semantics = foreground_painter
            .as_ref()
            .filter(|painter| painter.semantics_builder().is_some())
            .cloned();
        if child.is_none() && foreground_painter.is_none() && background_semantics.is_none() {
            return Widget::from_kind(WidgetKind::CustomPaint {
                size,
                display_list,
                painter,
                foreground: false,
            });
        }
        // Stack siblings keep Flutter's order: painter, child, foreground.
        // Painter regions sit with their painter, so semantic order is
        // background regions, child, foreground regions
        // (`RenderCustomPaint.assembleSemanticsNode`).
        // The painter layers fill the stack, which takes the child's size.
        let layer = |painter: Option<PainterDelegate>, display_list: DisplayList, foreground: bool| -> Widget {
            Positioned::fill(Widget::from_kind(WidgetKind::CustomPaint {
                size: None,
                display_list,
                painter,
                foreground,
            }))
            .into()
        };
        let has_child = child.is_some();
        // RenderCustomPaint introduces one accessibility boundary only when
        // at least one of its painters supplies regions.
        let has_region_boundary = background_semantics.is_some() || foreground_semantics.is_some();
        let mut children: Vec<Widget> = Vec::new();
        if painter.is_some() || !display_list.is_empty() {
            children.push(layer(painter, display_list, false));
        }
        children.extend(background_semantics.map(|painter| semantics_layer(painter, false)));
        children.extend(child);
        if let Some(foreground) = foreground_painter {
            children.push(layer(Some(foreground), DisplayList::default(), true));
        }
        children.extend(foreground_semantics.map(|painter| semantics_layer(painter, true)));
        let stack: Widget = Stack::new(children)
            .fit(StackFit::Passthrough)
            .clip_behavior(Clip::None)
            .into();
        let stack = if has_region_boundary {
            Widget::from(crate::Semantics::new(stack).container(true)).with_semantic_child_order()
        } else {
            stack
        };
        match size {
            Some(size) if !has_child => SizedBox::new()
                .width(size.width)
                .height(size.height)
                .child(stack)
                .into(),
            _ => stack,
        }
    }
}

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
    #[builder(default, setter(strip_option, into))]
    child: Option<Widget>,
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
            child: None,
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

    /// Paints the outline behind a child without blocking its hit targets.
    #[must_use]
    pub fn child(mut self, child: impl Into<Widget>) -> Self {
        self.child = Some(child.into());
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

    fn hit_test(&self, _position: Offset) -> Option<bool> {
        Some(false)
    }
}

impl From<Placeholder> for Widget {
    fn from(value: Placeholder) -> Self {
        let painter = PlaceholderPainter {
            color: value.color,
            stroke_width: value.stroke_width,
        };
        let paint = CustomPaint::from_painter_filling(painter);
        let paint = if let Some(child) = value.child {
            paint.child(child)
        } else {
            paint
        };
        LimitedBox::new(paint)
            .max_width(value.fallback_width)
            .max_height(value.fallback_height)
            .into()
    }
}

/// Saved separately from the painter's display list. Repainting does
/// not rebuild semantic regions; a change in size always does.
#[derive(Default)]
struct PainterRegionsState(RefCell<Option<(Size, bool, PainterDelegate, Widget)>>);

impl crate::tree::RetainedState for PainterRegionsState {
    fn rebind_revision(&self, _: Rc<Cell<u64>>) {}
}

fn semantics_layer(painter: PainterDelegate, foreground: bool) -> Widget {
    // These generated regions use empty boxes and cannot claim a pointer.
    // IgnorePointer would also strip their real semantic callbacks.
    Positioned::fill(Widget::mount_scoped_layout_builder(
        |_| PainterRegionsState::default(),
        move |state, _, constraints| {
            let size = constraints.biggest();
            let mut cached = state.0.borrow_mut();
            if let Some((old_size, old_foreground, old, regions)) = cached.as_mut() {
                let same_role = *old_foreground == foreground;
                let refresh = !same_role
                    || *old != painter && painter.should_rebuild_semantics_over(old);
                if !refresh && *old_size == size {
                    // Flutter compares against the actual replacement next
                    // time, even when it elects to keep the old regions.
                    *old = painter.clone();
                    return regions.clone();
                }
            }
            let regions = painter.semantics_builder().into_iter().flat_map(|builder| builder(size))
                .map(|region| {
                    let (rect, key, semantics) = region.into_parts();
                    let node: Widget = Positioned::new(semantics.with_child(SizedBox::new()))
                        .left(rect.origin.x)
                        .top(rect.origin.y)
                        .width(rect.size.width.max(0.))
                        .height(rect.size.height.max(0.))
                        .into();
                    match key {
                        Some(key) => node.with_key(key),
                        None => node,
                    }
                });
            let regions: Widget = Stack::new(regions).clip_behavior(Clip::None).into();
            *cached = Some((size, foreground, painter.clone(), regions.clone()));
            regions
        },
        true,
    ))
    .into()
}
