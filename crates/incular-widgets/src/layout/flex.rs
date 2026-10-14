//! Flutter-style flex layout widget descriptors.
//!
//! [`Row`], [`Column`], and [`Flex`] configure proportional or fixed main-axis
//! distribution with cross-axis alignment, text direction, and inter-item spacing.
//! Layout algorithms are authoritative in `incular-layout`.

use incular_config::{
    Axis, CrossAxisAlignment, FlexFit, MainAxisAlignment, MainAxisSize, TextDirection,
    VerticalDirection,
};
use smart_default::SmartDefault;
use typed_builder::TypedBuilder;

use crate::{SizedBox, Widget, WidgetKind, macros::widget_from_kind};

/// Declares a fixed-axis flex container ([`Row`] or [`Column`]); the two
/// differ only in their axis.
macro_rules! axis_flex {
    ($name:ident, $axis:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(Clone, Debug, PartialEq, TypedBuilder, SmartDefault)]
        pub struct $name {
            #[builder(
                        default = Vec::new(),
                        setter(transform = |children: impl IntoIterator<Item = impl Into<Widget>>| {
                            children.into_iter().map(Into::into).collect::<Vec<Widget>>()
                        })
                    )]
            pub children: Vec<Widget>,
            #[builder(default = MainAxisAlignment::Start)]
            pub main_axis_alignment: MainAxisAlignment,
            #[builder(default = MainAxisSize::Max)]
            pub main_axis_size: MainAxisSize,
            #[builder(default = CrossAxisAlignment::Center)]
            #[default(CrossAxisAlignment::Center)]
            pub cross_axis_alignment: CrossAxisAlignment,
            #[builder(default = None, setter(strip_option))]
            pub text_direction: Option<TextDirection>,
            #[builder(default = VerticalDirection::Down)]
            pub vertical_direction: VerticalDirection,
            #[builder(
                        default = 0.0,
                        setter(transform = |spacing: f32| spacing.max(0.0))
                    )]
            pub spacing: f32,
        }

        impl $name {
            /// Creates the flex container.
            #[must_use]
            pub fn new(children: impl IntoIterator<Item = impl Into<Widget>>) -> Self {
                Self {
                    children: children.into_iter().map(Into::into).collect(),
                    ..Self::default()
                }
            }

            /// Replaces the children.
            #[must_use]
            pub fn children(
                mut self,
                children: impl IntoIterator<Item = impl Into<Widget>>,
            ) -> Self {
                self.children = children.into_iter().map(Into::into).collect();
                self
            }

            /// Sets the main-axis alignment policy.
            #[must_use]
            pub fn main_axis_alignment(mut self, alignment: MainAxisAlignment) -> Self {
                self.main_axis_alignment = alignment;
                self
            }

            /// Sets whether the main axis should expand to parent max or shrink to content.
            #[must_use]
            pub fn main_axis_size(mut self, size: MainAxisSize) -> Self {
                self.main_axis_size = size;
                self
            }

            /// Sets the cross-axis alignment policy.
            #[must_use]
            pub fn cross_axis_alignment(mut self, alignment: CrossAxisAlignment) -> Self {
                self.cross_axis_alignment = alignment;
                self
            }

            /// Sets cross-axis alignment (convenience alias for [`Self::cross_axis_alignment`]).
            #[must_use]
            pub fn alignment(self, alignment: CrossAxisAlignment) -> Self {
                self.cross_axis_alignment(alignment)
            }

            /// Sets the reading/layout direction along the horizontal axis.
            #[must_use]
            pub fn text_direction(mut self, direction: TextDirection) -> Self {
                self.text_direction = Some(direction);
                self
            }

            /// Sets the vertical direction.
            #[must_use]
            pub fn vertical_direction(mut self, direction: VerticalDirection) -> Self {
                self.vertical_direction = direction;
                self
            }

            /// Sets the fixed spacing gap between consecutive children.
            #[must_use]
            pub fn spacing(mut self, spacing: f32) -> Self {
                self.spacing = spacing.max(0.0);
                self
            }

            #[must_use]
            pub fn get_main_axis_alignment(&self) -> MainAxisAlignment {
                self.main_axis_alignment
            }

            #[must_use]
            pub fn get_main_axis_size(&self) -> MainAxisSize {
                self.main_axis_size
            }

            #[must_use]
            pub fn get_cross_axis_alignment(&self) -> CrossAxisAlignment {
                self.cross_axis_alignment
            }

            #[must_use]
            pub fn get_spacing(&self) -> f32 {
                self.spacing.max(0.0)
            }
        }

        impl From<$name> for Widget {
            fn from(value: $name) -> Self {
                Widget::from_kind(WidgetKind::Flex {
                    axis: Axis::$axis,
                    main_axis_alignment: value.main_axis_alignment,
                    main_axis_size: value.main_axis_size,
                    cross_axis_alignment: value.cross_axis_alignment,
                    text_direction: value.text_direction,
                    vertical_direction: value.vertical_direction,
                    spacing: value.spacing.max(0.0),
                    children: value.children,
                })
            }
        }
    };
}

axis_flex!(Row, Horizontal, "A horizontal flex container.");
axis_flex!(Column, Vertical, "A vertical flex container.");

/// A direction-configurable flex container.
#[derive(Clone, Debug, PartialEq, TypedBuilder, SmartDefault)]
pub struct Flex {
    #[builder(default = Axis::Horizontal)]
    pub direction: Axis,
    #[builder(
        default = Vec::new(),
        setter(transform = |children: impl IntoIterator<Item = impl Into<Widget>>| {
            children.into_iter().map(Into::into).collect::<Vec<Widget>>()
        })
    )]
    pub children: Vec<Widget>,
    #[builder(default = MainAxisAlignment::Start)]
    pub main_axis_alignment: MainAxisAlignment,
    #[builder(default = MainAxisSize::Max)]
    pub main_axis_size: MainAxisSize,
    #[builder(default = CrossAxisAlignment::Center)]
    #[default(CrossAxisAlignment::Center)]
    pub cross_axis_alignment: CrossAxisAlignment,
    #[builder(default = None, setter(strip_option))]
    pub text_direction: Option<TextDirection>,
    #[builder(default = VerticalDirection::Down)]
    pub vertical_direction: VerticalDirection,
    #[builder(
        default = 0.0,
        setter(transform = |spacing: f32| spacing.max(0.0))
    )]
    pub spacing: f32,
}

impl Flex {
    /// Creates a flex container with explicit direction.
    #[must_use]
    pub fn new(direction: Axis, children: impl IntoIterator<Item = impl Into<Widget>>) -> Self {
        Self {
            direction,
            children: children.into_iter().map(Into::into).collect(),
            ..Self::default()
        }
    }

    #[must_use]
    pub fn horizontal(children: impl IntoIterator<Item = impl Into<Widget>>) -> Self {
        Self::new(Axis::Horizontal, children)
    }

    #[must_use]
    pub fn vertical(children: impl IntoIterator<Item = impl Into<Widget>>) -> Self {
        Self::new(Axis::Vertical, children)
    }

    #[must_use]
    pub fn direction(mut self, direction: Axis) -> Self {
        self.direction = direction;
        self
    }

    #[must_use]
    pub fn main_axis_alignment(mut self, alignment: MainAxisAlignment) -> Self {
        self.main_axis_alignment = alignment;
        self
    }

    #[must_use]
    pub fn main_axis_size(mut self, size: MainAxisSize) -> Self {
        self.main_axis_size = size;
        self
    }

    #[must_use]
    pub fn cross_axis_alignment(mut self, alignment: CrossAxisAlignment) -> Self {
        self.cross_axis_alignment = alignment;
        self
    }

    #[must_use]
    pub fn text_direction(mut self, direction: TextDirection) -> Self {
        self.text_direction = Some(direction);
        self
    }

    #[must_use]
    pub fn vertical_direction(mut self, direction: VerticalDirection) -> Self {
        self.vertical_direction = direction;
        self
    }

    #[must_use]
    pub fn spacing(mut self, spacing: f32) -> Self {
        self.spacing = spacing.max(0.0);
        self
    }
}

impl From<Flex> for Widget {
    fn from(value: Flex) -> Self {
        Widget::from_kind(WidgetKind::Flex {
            axis: value.direction,
            main_axis_alignment: value.main_axis_alignment,
            main_axis_size: value.main_axis_size,
            cross_axis_alignment: value.cross_axis_alignment,
            text_direction: value.text_direction,
            vertical_direction: value.vertical_direction,
            spacing: value.spacing.max(0.0),
            children: value.children,
        })
    }
}

/// Allocates a proportional share of a bounded `Row` or `Column` main axis.
#[derive(Clone, Debug, PartialEq, TypedBuilder)]
pub struct Flexible {
    #[builder(default = 1)]
    pub flex: u32,
    #[builder(default = FlexFit::Loose)]
    pub fit: FlexFit,
    #[builder(setter(into))]
    pub child: Widget,
}

impl Flexible {
    /// Creates a flexible wrapper around a child.
    #[must_use]
    pub fn new(child: impl Into<Widget>) -> Self {
        Self {
            flex: 1,
            fit: FlexFit::Loose,
            child: child.into(),
        }
    }

    /// Convenience constructor with explicit flex factor. Zero makes the
    /// child inflexible, as in Flutter.
    #[must_use]
    pub fn flex_factor(flex: u32, child: impl Into<Widget>) -> Self {
        Self {
            flex,
            fit: FlexFit::Loose,
            child: child.into(),
        }
    }

    /// Sets the flex factor share. Zero makes the child inflexible.
    #[must_use]
    pub fn flex(mut self, flex: u32) -> Self {
        self.flex = flex;
        self
    }

    /// Sets tight flex fit.
    #[must_use]
    pub fn tight(mut self) -> Self {
        self.fit = FlexFit::Tight;
        self
    }

    /// Sets the flex fit policy.
    #[must_use]
    pub fn fit(mut self, fit: FlexFit) -> Self {
        self.fit = fit;
        self
    }
}

widget_from_kind!(Flexible => Flexible { flex, fit, child });

/// A tight flexible child occupying its proportional share of available flex space.
#[derive(Clone, Debug, PartialEq, TypedBuilder)]
pub struct Expanded {
    #[builder(default = 1)]
    pub flex: u32,
    #[builder(setter(into))]
    pub child: Widget,
}

impl Expanded {
    /// Creates an expanded child with a default flex of 1.
    #[must_use]
    pub fn new(child: impl Into<Widget>) -> Self {
        Self {
            flex: 1,
            child: child.into(),
        }
    }

    /// Convenience constructor with explicit flex factor. Zero makes the
    /// child inflexible, as in Flutter.
    #[must_use]
    pub fn flex_factor(flex: u32, child: impl Into<Widget>) -> Self {
        Self {
            flex,
            child: child.into(),
        }
    }

    /// Sets the flex factor share. Zero makes the child inflexible.
    #[must_use]
    pub fn flex(mut self, flex: u32) -> Self {
        self.flex = flex;
        self
    }
}

impl From<Expanded> for Widget {
    fn from(value: Expanded) -> Self {
        Widget::from_kind(WidgetKind::Flexible {
            flex: value.flex,
            fit: FlexFit::Tight,
            child: value.child,
        })
    }
}

/// An empty flexible space on a flex axis.
#[derive(Clone, Copy, Debug, PartialEq, Eq, TypedBuilder)]
pub struct Spacer {
    #[builder(
        default = 1,
        setter(transform = |flex: u32| {
            // Flutter's `Spacer` asserts `flex > 0` (widgets/spacer.dart).
            debug_assert!(flex > 0, "Spacer flex must be greater than zero");
            flex
        })
    )]
    pub flex: u32,
}

impl Default for Spacer {
    fn default() -> Self {
        Self::new()
    }
}

impl Spacer {
    /// Creates a spacer with a default flex of 1.
    #[must_use]
    pub const fn new() -> Self {
        Self { flex: 1 }
    }

    /// Convenience constructor with explicit flex factor.
    ///
    /// Like Flutter's `Spacer`, a zero flex is a programming error
    /// (`assert(flex > 0)`): it panics in debug builds.
    #[must_use]
    pub const fn flex_factor(flex: u32) -> Self {
        debug_assert!(flex > 0, "Spacer flex must be greater than zero");
        Self { flex }
    }

    /// Sets the flex factor share. A zero flex panics in debug builds, as
    /// Flutter's `Spacer` asserts `flex > 0`.
    #[must_use]
    pub const fn flex(mut self, flex: u32) -> Self {
        debug_assert!(flex > 0, "Spacer flex must be greater than zero");
        self.flex = flex;
        self
    }
}

impl From<Spacer> for Widget {
    fn from(value: Spacer) -> Self {
        Expanded::new(SizedBox::shrink()).flex(value.flex).into()
    }
}
