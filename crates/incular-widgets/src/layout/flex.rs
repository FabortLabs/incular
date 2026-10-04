//! Flutter-style flex layout widget descriptors.
//!
//! [`Row`], [`Column`], and [`Flex`] configure proportional or fixed main-axis
//! distribution with cross-axis alignment, text direction, and inter-item spacing.
//! Layout algorithms are authoritative in `incular-layout`.

use incular_config::{
    Axis, CrossAxisAlignment, FlexFit, MainAxisAlignment, MainAxisSize, TextDirection,
    VerticalDirection,
};
use typed_builder::TypedBuilder;

use crate::{SizedBox, Widget, WidgetKind};

/// Declares a fixed-axis flex container ([`Row`] or [`Column`]); the two
/// differ only in their axis.
macro_rules! axis_flex {
    ($name:ident, $axis:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(Clone, Debug, PartialEq, TypedBuilder)]
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

        impl Default for $name {
            fn default() -> Self {
                Self {
                    children: Vec::new(),
                    main_axis_alignment: MainAxisAlignment::Start,
                    main_axis_size: MainAxisSize::Max,
                    cross_axis_alignment: CrossAxisAlignment::Center,
                    text_direction: None,
                    vertical_direction: VerticalDirection::Down,
                    spacing: 0.0,
                }
            }
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
                    text_direction: value.text_direction.unwrap_or(TextDirection::Ltr),
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
#[derive(Clone, Debug, PartialEq, TypedBuilder)]
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

impl Default for Flex {
    fn default() -> Self {
        Self {
            direction: Axis::Horizontal,
            children: Vec::new(),
            main_axis_alignment: MainAxisAlignment::Start,
            main_axis_size: MainAxisSize::Max,
            cross_axis_alignment: CrossAxisAlignment::Center,
            text_direction: None,
            vertical_direction: VerticalDirection::Down,
            spacing: 0.0,
        }
    }
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
            text_direction: value.text_direction.unwrap_or(TextDirection::Ltr),
            vertical_direction: value.vertical_direction,
            spacing: value.spacing.max(0.0),
            children: value.children,
        })
    }
}

/// Allocates a proportional share of a bounded `Row` or `Column` main axis.
#[derive(Clone, Debug, PartialEq, TypedBuilder)]
pub struct Flexible {
    #[builder(
        default = 1,
        setter(transform = |flex: u32| flex.max(1))
    )]
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

    /// Convenience constructor with explicit flex factor.
    #[must_use]
    pub fn flex_factor(flex: u32, child: impl Into<Widget>) -> Self {
        Self {
            flex: flex.max(1),
            fit: FlexFit::Loose,
            child: child.into(),
        }
    }

    /// Sets the flex factor share.
    #[must_use]
    pub fn flex(mut self, flex: u32) -> Self {
        self.flex = flex.max(1);
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

impl From<Flexible> for Widget {
    fn from(value: Flexible) -> Self {
        Widget::from_kind(WidgetKind::Flexible {
            flex: value.flex.max(1),
            fit: value.fit,
            child: value.child,
        })
    }
}

/// A tight flexible child occupying its proportional share of available flex space.
#[derive(Clone, Debug, PartialEq, TypedBuilder)]
pub struct Expanded {
    #[builder(
        default = 1,
        setter(transform = |flex: u32| flex.max(1))
    )]
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

    /// Convenience constructor with explicit flex factor.
    #[must_use]
    pub fn flex_factor(flex: u32, child: impl Into<Widget>) -> Self {
        Self {
            flex: flex.max(1),
            child: child.into(),
        }
    }

    /// Sets the flex factor share.
    #[must_use]
    pub fn flex(mut self, flex: u32) -> Self {
        self.flex = flex.max(1);
        self
    }
}

impl From<Expanded> for Widget {
    fn from(value: Expanded) -> Self {
        Widget::from_kind(WidgetKind::Flexible {
            flex: value.flex.max(1),
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
        setter(transform = |flex: u32| flex.max(1))
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
    #[must_use]
    pub const fn flex_factor(flex: u32) -> Self {
        Self {
            flex: if flex == 0 { 1 } else { flex },
        }
    }

    /// Sets the flex factor share.
    #[must_use]
    pub const fn flex(mut self, flex: u32) -> Self {
        self.flex = if flex == 0 { 1 } else { flex };
        self
    }
}

impl From<Spacer> for Widget {
    fn from(value: Spacer) -> Self {
        Expanded::new(SizedBox::shrink())
            .flex(value.flex.max(1))
            .into()
    }
}
