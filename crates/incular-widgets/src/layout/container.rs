//! Flutter-style Container composition widget.
//!
//! [`Container`] is a convenience composition widget that combines common
//! painting, positioning, sizing, padding, margins, and transforms into a single
//! ergonomic descriptor. It lowers to existing primitive widgets without introducing
//! a duplicate layout or rendering engine.

use std::rc::Rc;

use incular_config::{Alignment, Clip, Constraints, EdgeInsets};
use incular_core::{Color, Transform as CoreTransform};
use incular_rendering::{Border, Brush, CornerRadii};
use smart_default::SmartDefault;
use typed_builder::TypedBuilder;

use crate::layout::basic::{
    Align, ClipRRect, ClipRect, ConstrainedBox, FractionallySizedBox, Padding, SizedBox,
};
use crate::{AlignmentGeometry, Transform, Widget};

/// A convenience composition widget combining sizing, constraints, margin,
/// padding, background color/decoration, alignment, transforms, and clipping.
///
/// Like Flutter's `Container`, it does not clip unless
/// [`Container::clip_behavior`] opts in: an implicit clip would add a clip
/// widget, compositor layers and (for rounded corners) a stencil mask to
/// every decorated box.
#[derive(Clone, Debug, PartialEq, TypedBuilder, SmartDefault)]
pub struct Container {
    #[builder(default, setter(strip_option, into))]
    pub child: Option<Widget>,
    #[builder(default, setter(strip_option))]
    pub alignment: Option<Alignment>,
    #[builder(default, setter(strip_option))]
    pub padding: Option<EdgeInsets>,
    #[builder(default, setter(strip_option))]
    pub margin: Option<EdgeInsets>,
    #[builder(default, setter(strip_option))]
    pub color: Option<Color>,
    #[builder(default, setter(strip_option, into))]
    pub background: Option<Brush>,
    #[builder(default, setter(strip_option, into))]
    pub border: Option<Border>,
    #[builder(default, setter(into))]
    pub radius: CornerRadii,
    #[builder(default, setter(strip_option))]
    pub constraints: Option<Constraints>,
    #[builder(default, setter(strip_option))]
    pub width: Option<f32>,
    #[builder(default, setter(strip_option))]
    pub height: Option<f32>,
    #[builder(default, setter(strip_option))]
    pub transform: Option<CoreTransform>,
    /// Where the transform pivots within the outer box, including its margin.
    /// The default pivot is the top-left corner.
    #[builder(default, setter(strip_option, into))]
    pub transform_alignment: Option<AlignmentGeometry>,
    /// Background decoration, resolved identically for the builder, fluent
    /// API, and struct literals.
    #[builder(default, setter(strip_option))]
    pub decoration: Option<crate::BoxDecoration>,
    /// Flutter `Container.foregroundDecoration`: painted in front of the
    /// child.
    #[builder(default, setter(strip_option))]
    pub foreground_decoration: Option<crate::BoxDecoration>,
    #[default(Clip::None)]
    #[builder(default = Clip::None)]
    pub clip_behavior: Clip,
}

impl Container {
    /// Creates a new empty Container.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a new Container wrapping a child widget.
    #[must_use]
    pub fn with_child(child: impl Into<Widget>) -> Self {
        Self {
            child: Some(child.into()),
            ..Self::default()
        }
    }

    /// Creates an empty Container.
    #[must_use]
    pub fn empty() -> Self {
        Self::default()
    }

    /// Sets the child widget.
    #[must_use]
    pub fn child(mut self, child: impl Into<Widget>) -> Self {
        self.child = Some(child.into());
        self
    }

    /// Sets inner alignment of the child within container bounds.
    #[must_use]
    pub fn alignment(mut self, alignment: Alignment) -> Self {
        self.alignment = Some(alignment);
        self
    }

    /// Sets inner padding.
    #[must_use]
    pub fn padding(mut self, padding: EdgeInsets) -> Self {
        self.padding = Some(padding);
        self
    }

    /// Sets outer margin.
    #[must_use]
    pub fn margin(mut self, margin: EdgeInsets) -> Self {
        self.margin = Some(margin);
        self
    }

    /// Sets background solid color.
    #[must_use]
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    /// Sets background brush (color, gradient, etc.).
    #[must_use]
    pub fn background(mut self, brush: impl Into<Brush>) -> Self {
        self.background = Some(brush.into());
        self
    }

    /// Sets container border.
    #[must_use]
    pub fn border(mut self, border: impl Into<Border>) -> Self {
        self.border = Some(border.into());
        self
    }

    /// Sets container visual decoration (color, border, radius).
    #[must_use]
    pub fn decoration(mut self, decoration: crate::BoxDecoration) -> Self {
        self.decoration = Some(decoration);
        self
    }

    /// Flutter `Container.foregroundDecoration`: a decoration painted in
    /// front of the child.
    #[must_use]
    pub fn foreground_decoration(mut self, decoration: crate::BoxDecoration) -> Self {
        self.foreground_decoration = Some(decoration);
        self
    }

    /// Sets container corner radii.
    #[must_use]
    pub fn radius(mut self, radius: impl Into<CornerRadii>) -> Self {
        self.radius = radius.into();
        self
    }

    /// Sets explicit fixed width.
    #[must_use]
    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    /// Sets explicit fixed height.
    #[must_use]
    pub fn height(mut self, height: f32) -> Self {
        self.height = Some(height);
        self
    }

    /// Sets additional layout constraints.
    #[must_use]
    pub fn constraints(mut self, constraints: Constraints) -> Self {
        self.constraints = Some(constraints);
        self
    }

    /// Sets transform matrix applied to the container.
    #[must_use]
    pub fn transform(mut self, transform: CoreTransform) -> Self {
        self.transform = Some(transform);
        self
    }

    /// Sets the physical or direction-relative pivot of the transform.
    #[must_use]
    pub fn transform_alignment(mut self, alignment: impl Into<AlignmentGeometry>) -> Self {
        self.transform_alignment = Some(alignment.into());
        self
    }

    /// Sets clipping behavior.
    #[must_use]
    pub fn clip_behavior(mut self, clip: Clip) -> Self {
        self.clip_behavior = clip;
        self
    }
}

impl From<Container> for Widget {
    fn from(value: Container) -> Self {
        if let Some(requirement) = value
            .decoration
            .as_ref()
            .and_then(|decoration| decoration.direction_requirement(true))
        {
            return crate::Builder::new(move |context| {
                let direction = context
                    .text_direction()
                    .expect("validated directional Container");
                Container::build_widget(value.clone(), Some(direction))
            })
            .require_directionality(requirement)
            .into();
        }
        Container::build_widget(value, None)
    }
}

impl Container {
    fn build_widget(mut value: Self, direction: Option<incular_config::TextDirection>) -> Widget {
        // Layout and child clipping need physical radii, but a full painter
        // keeps the original logical decoration as its cache identity.
        let logical_paint = if let Some(direction) = direction {
            let source = value.decoration.take().expect("directional decoration");
            value.decoration = Some(source.clone().resolved_for_direction(direction));
            (source.has_aligned_gradient() || source.directional_border.is_some())
                .then_some(source)
        } else {
            None
        };
        let non_negative = |value: Option<f32>| value.map(|value| value.max(0.0));
        let constraints = value.constraints.map(|constraints| {
            constraints.tighten(non_negative(value.width), non_negative(value.height))
        });
        let tightly_sized = constraints.map_or_else(
            || value.width.is_some() && value.height.is_some(),
            |bounds| {
                bounds.min_width() == bounds.max_width()
                    && bounds.min_height() == bounds.max_height()
            },
        );
        let mut current = match value.child {
            None if !tightly_sized => {
                // Flutter uses LimitedBox(0, 0) around an expanding child.
                // A childless one-to-one fill takes the finite maxima or
                // the incoming minima on unbounded axes without a dummy leaf.
                FractionallySizedBox::empty()
                    .width_factor(1.0)
                    .height_factor(1.0)
                    .into()
            }
            child => {
                let child = child.unwrap_or_else(|| SizedBox::shrink().into());
                match value.alignment {
                    Some(alignment) => Align::new(alignment, child).into(),
                    None => child,
                }
            }
        };

        let decoration = value.decoration.as_ref();
        let border = value.border.or_else(|| {
            decoration
                .and_then(|decoration| decoration.border)
                .filter(|border| border.top.style != crate::BorderStyle::None)
                .map(Into::into)
        });
        let radius = if value.radius.is_zero() {
            decoration
                .and_then(|decoration| decoration.border_radius)
                .map_or(value.radius, Into::into)
        } else {
            value.radius
        };

        // Padding is independent of painting. A decoration border can have
        // an inset even when its style is invisible.
        let border_padding = if let Some(decoration) = decoration {
            decoration.padding()
        } else {
            value
                .border
                .map_or(EdgeInsets::ZERO, |border| EdgeInsets::all(border.width))
        };
        let effective_padding = value.padding.unwrap_or(EdgeInsets::ZERO) + border_padding;
        if !effective_padding.is_zero() {
            current = Padding::new(effective_padding, current).into();
        }

        // Clip inside the decoration and outside the inner padding. A
        // shadow or outside stroke must not be cropped by a child clip.
        if value.clip_behavior != Clip::None
            && let Some(decoration) = decoration
        {
            current = decoration.clip_child(value.clip_behavior, current);
        }
        let standalone_clip = value.clip_behavior != Clip::None && decoration.is_none();

        // 3. Decoration, then the optional foreground over it. Stay on the
        // single render-object fast path for simple rounded rectangles.
        let background = value
            .color
            .map(Brush::Solid)
            .or(value.background)
            .or_else(|| {
                decoration.and_then(|decoration| {
                    decoration
                        .gradient
                        .clone()
                        .or_else(|| decoration.color.map(Brush::Solid))
                })
            });
        match value.decoration {
            Some(decoration)
                if decoration.needs_full_painter()
                    || logical_paint.as_ref().is_some_and(|source| source.directional_border.is_some()) =>
            {
                current = crate::painting_effects::decorated(
                    logical_paint.unwrap_or(decoration),
                    crate::DecorationPosition::Background,
                    current,
                );
            }
            decoration
                if decoration.is_some()
                    || background.is_some()
                    || border.is_some()
                    || !radius.is_zero() =>
            {
                current = Widget::decorated_with_hit(
                    None,
                    background,
                    border,
                    radius,
                    current,
                    decoration.map(Rc::new),
                );
            }
            _ => {}
        }
        if let Some(foreground) = value.foreground_decoration {
            current = crate::painting_effects::decorated(
                foreground,
                crate::DecorationPosition::Foreground,
                current,
            );
        }

        // 4. Sizing / Constraints, as Flutter's `Container` builds them:
        // `constraints.tighten(width, height)` when both are given, which
        // clamps the dimensions into the constraints, otherwise
        // `BoxConstraints.tightFor(width, height)`. An infinite dimension
        // (`width: double.infinity`) fills the parent's maximum, which is
        // what `SizedBox` does with it.
        match (constraints, value.width, value.height) {
            (Some(constraints), _, _) => {
                if constraints != Constraints::unbounded() {
                    current = ConstrainedBox::new(constraints, current).into();
                }
            }
            (None, None, None) => {}
            (None, width, height) => {
                current = SizedBox::from_dimensions(width, height, Some(current)).into();
            }
        }

        // 5. Margin (outer padding)
        if let Some(margin) = value.margin
            && !margin.is_zero()
        {
            current = Padding::new(margin, current).into();
        }

        // 6. Transform
        if let Some(transform) = value.transform {
            let transformed = Transform::new(transform, current);
            current = match value.transform_alignment {
                Some(alignment) => transformed.alignment(alignment).into(),
                None => transformed.into(),
            };
        }

        // The Incular-only shorthand without a BoxDecoration remains an
        // outer rectangle/rounded-rectangle clip.
        if standalone_clip {
            current = if radius.is_zero() {
                ClipRect::new(current)
                    .clip_behavior(value.clip_behavior)
                    .into()
            } else {
                ClipRRect::new(radius, current)
                    .clip_behavior(value.clip_behavior)
                    .into()
            };
        }

        current
    }
}
