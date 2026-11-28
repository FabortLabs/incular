//! Flutter-style multi-run Wrap layout descriptor.

use incular_config::{
    Axis, Clip, TextDirection, VerticalDirection, WrapAlignment, WrapCrossAlignment,
};
use incular_core::{DiagnosticEnum, DiagnosticValue, DiagnosticsNode, DiagnosticsProperty};
use smart_default::SmartDefault;
use typed_builder::TypedBuilder;

use crate::{Widget, WidgetKind};

/// A multi-run layout widget that places children sequentially, wrapping onto
/// new runs when exceeding available main-axis extent.
#[derive(Clone, Debug, PartialEq, TypedBuilder, SmartDefault)]
pub struct Wrap {
    #[builder(default, setter(into))]
    children: Vec<Widget>,
    #[builder(default = Axis::Horizontal)]
    direction: Axis,
    #[builder(default = WrapAlignment::Start)]
    alignment: WrapAlignment,
    #[builder(default, setter(transform = |spacing: f32| spacing.max(0.0)))]
    spacing: f32,
    #[builder(default = WrapAlignment::Start)]
    run_alignment: WrapAlignment,
    #[builder(default, setter(transform = |spacing: f32| spacing.max(0.0)))]
    run_spacing: f32,
    #[builder(default = WrapCrossAlignment::Start)]
    cross_axis_alignment: WrapCrossAlignment,
    #[builder(default, setter(strip_option))]
    text_direction: Option<TextDirection>,
    #[builder(default = VerticalDirection::Down)]
    vertical_direction: VerticalDirection,
    #[default(Clip::None)]
    #[builder(default = Clip::None)]
    clip_behavior: Clip,
}

impl Wrap {
    /// Creates a new Wrap layout.
    #[must_use]
    pub fn new(children: impl IntoIterator<Item = impl Into<Widget>>) -> Self {
        Self {
            children: children.into_iter().map(Into::into).collect(),
            ..Self::default()
        }
    }

    /// Sets the wrapping main axis.
    #[must_use]
    pub fn direction(mut self, direction: Axis) -> Self {
        self.direction = direction;
        self
    }

    /// Sets how children within a single run align on the main axis.
    #[must_use]
    pub fn alignment(mut self, alignment: WrapAlignment) -> Self {
        self.alignment = alignment;
        self
    }

    /// Sets the spacing between children in the same run.
    #[must_use]
    pub fn spacing(mut self, spacing: f32) -> Self {
        self.spacing = spacing.max(0.0);
        self
    }

    /// Sets how runs align along the cross axis.
    #[must_use]
    pub fn run_alignment(mut self, alignment: WrapAlignment) -> Self {
        self.run_alignment = alignment;
        self
    }

    /// Sets the spacing between runs.
    #[must_use]
    pub fn run_spacing(mut self, spacing: f32) -> Self {
        self.run_spacing = spacing.max(0.0);
        self
    }

    /// Sets how children within a run align relative to each other on the cross axis.
    #[must_use]
    pub fn cross_axis_alignment(mut self, alignment: WrapCrossAlignment) -> Self {
        self.cross_axis_alignment = alignment;
        self
    }

    /// Sets the text direction for horizontal runs.
    #[must_use]
    pub fn text_direction(mut self, direction: TextDirection) -> Self {
        self.text_direction = Some(direction);
        self
    }

    /// Sets the vertical direction for vertical runs.
    #[must_use]
    pub fn vertical_direction(mut self, direction: VerticalDirection) -> Self {
        self.vertical_direction = direction;
        self
    }

    /// Clips painted overflow, without clipping children that fit.
    /// Defaults to `Clip::None`, including when the runs overflow.
    #[must_use]
    pub fn clip_behavior(mut self, behavior: Clip) -> Self {
        self.clip_behavior = behavior;
        self
    }

    #[must_use]
    pub fn get_direction(&self) -> Axis {
        self.direction
    }

    #[must_use]
    pub fn get_spacing(&self) -> f32 {
        self.spacing
    }

    #[must_use]
    pub fn get_run_spacing(&self) -> f32 {
        self.run_spacing
    }
}

// These local adapters keep Flutter's inspector spelling and wire types
// separate from Rust's Debug spellings and the layout's hot path.
macro_rules! enum_diagnostic {
    ($adapter:ident, $value:ident { $($variant:ident => $name:literal),+ $(,)? }) => {
        #[derive(Clone, Copy, PartialEq)]
        struct $adapter($value);

        impl DiagnosticValue for $adapter {
            const TYPE_NAME: &'static str = stringify!($value);

            fn to_diagnostic_json(&self) -> serde_json::Value {
                self.variant_name().into()
            }

            fn diagnostic_text(&self) -> String {
                format!("{}.{}", Self::TYPE_NAME, self.variant_name())
            }
        }

        impl DiagnosticEnum for $adapter {
            fn variant_name(&self) -> &'static str {
                match self.0 { $($value::$variant => $name),+ }
            }
        }
    };
}

enum_diagnostic!(DiagnosticAxis, Axis {
    Horizontal => "horizontal", Vertical => "vertical",
});
enum_diagnostic!(DiagnosticWrapAlignment, WrapAlignment {
    Start => "start", End => "end", Center => "center",
    SpaceBetween => "spaceBetween", SpaceAround => "spaceAround", SpaceEvenly => "spaceEvenly",
});
enum_diagnostic!(DiagnosticWrapCrossAlignment, WrapCrossAlignment {
    Start => "start", End => "end", Center => "center",
});
enum_diagnostic!(DiagnosticTextDirection, TextDirection {
    Ltr => "ltr", Rtl => "rtl",
});
enum_diagnostic!(DiagnosticVerticalDirection, VerticalDirection {
    Down => "down", Up => "up",
});

impl Wrap {
    /// Captures the properties reported by the widget inspector without
    /// mounting the widget or inspecting its children.
    #[must_use]
    pub fn to_diagnostics(&self) -> DiagnosticsNode {
        fn enumeration<T: DiagnosticEnum>(name: &str, value: T) -> DiagnosticsNode {
            DiagnosticsNode::property(&DiagnosticsProperty::enumeration(name, Some(value)))
        }
        fn spacing(name: &str, value: f32) -> DiagnosticsNode {
            DiagnosticsNode::property(&DiagnosticsProperty::double(name, Some(value.into())))
        }
        let text_direction = DiagnosticsProperty::enumeration(
            "textDirection",
            self.text_direction.map(DiagnosticTextDirection),
        )
        .default_value(None);
        let vertical_direction = DiagnosticsProperty::enumeration(
            "verticalDirection",
            Some(DiagnosticVerticalDirection(self.vertical_direction)),
        )
        .default_value(Some(DiagnosticVerticalDirection(VerticalDirection::Down)));
        DiagnosticsNode::new("Wrap", "DiagnosticableTreeNode").with_properties(vec![
            enumeration("direction", DiagnosticAxis(self.direction)),
            enumeration("alignment", DiagnosticWrapAlignment(self.alignment)),
            spacing("spacing", self.spacing),
            enumeration("runAlignment", DiagnosticWrapAlignment(self.run_alignment)),
            spacing("runSpacing", self.run_spacing),
            enumeration(
                "crossAxisAlignment",
                DiagnosticWrapCrossAlignment(self.cross_axis_alignment),
            ),
            DiagnosticsNode::property(&text_direction),
            DiagnosticsNode::property(&vertical_direction),
        ])
    }
}

impl From<Wrap> for Widget {
    fn from(value: Wrap) -> Self {
        Widget::from_kind(WidgetKind::Wrap {
            axis: value.direction,
            alignment: value.alignment,
            spacing: value.spacing,
            run_alignment: value.run_alignment,
            run_spacing: value.run_spacing,
            cross_axis_alignment: value.cross_axis_alignment,
            text_direction: value.text_direction,
            vertical_direction: value.vertical_direction,
            clip_behavior: value.clip_behavior,
            children: value.children,
        })
    }
}
