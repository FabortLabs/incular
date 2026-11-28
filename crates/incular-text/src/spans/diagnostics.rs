use incular_core::{
    Color, DiagnosticLevel, DiagnosticsNode, DiagnosticsProperty, DiagnosticsTreeStyle,
};
use incular_gestures::MouseCursor;

use crate::{LineHeight, TextDecoration, TextStyle};

use super::{InlineSpan, TextSpan, WidgetSpan};

fn message(name: &str, text: impl Into<String>) -> DiagnosticsNode {
    DiagnosticsNode::property(&DiagnosticsProperty::message_property(name, text))
}

fn color(name: &str, value: Color) -> DiagnosticsNode {
    DiagnosticsNode::property(&DiagnosticsProperty::color(name, Some(value)))
}

fn number(name: &str, value: f32) -> DiagnosticsNode {
    DiagnosticsNode::property(&DiagnosticsProperty::double(name, Some(f64::from(value))))
}

fn uncapitalize(value: impl std::fmt::Debug) -> String {
    let mut value = format!("{value:?}");
    if let Some(capital) = value.get_mut(..1) {
        capital.make_ascii_lowercase();
    }
    value
}

fn decoration_name(decoration: TextDecoration) -> String {
    let mut names = Vec::new();
    for (flag, name) in [
        (TextDecoration::UNDERLINE, "underline"),
        (TextDecoration::OVERLINE, "overline"),
        (TextDecoration::LINE_THROUGH, "lineThrough"),
    ] {
        if decoration.contains(flag) {
            names.push(name);
        }
    }
    match names.as_slice() {
        [] => "TextDecoration.none".into(),
        [name] => format!("TextDecoration.{name}"),
        _ => format!("TextDecoration.combine([{}])", names.join(", ")),
    }
}

impl TextStyle {
    /// Returns the ordered visible style properties used by span diagnostics.
    #[must_use]
    pub fn diagnostic_properties(&self) -> Vec<DiagnosticsNode> {
        let mut fields = Vec::new();
        if self.has_explicit_color {
            fields.push(color("color", self.color));
        }
        if let Some(value) = self.background_color {
            fields.push(color("backgroundColor", value));
        }
        if self.has_explicit_family {
            fields.push(message("family", self.family.name()));
        }
        if self.has_explicit_fallback_families {
            fields.push(DiagnosticsNode::property(&DiagnosticsProperty::iterable(
                "familyFallback",
                Some(
                    self.fallback_families
                        .iter()
                        .map(|f| f.name().to_owned())
                        .collect(),
                ),
            )));
        }
        if self.has_explicit_size {
            fields.push(number("size", self.size));
        }
        if self.has_explicit_weight {
            fields.push(DiagnosticsNode::property(&DiagnosticsProperty::integer(
                "weight",
                Some(i64::from(self.weight.value())),
            )));
        }
        if self.has_explicit_style {
            fields.push(message("style", uncapitalize(self.style)));
        }
        if self.has_explicit_letter_spacing {
            fields.push(number("letterSpacing", self.letter_spacing));
        }
        if let Some(value) = self.word_spacing {
            fields.push(number("wordSpacing", value));
        }
        if let Some(value) = self.text_baseline {
            fields.push(message("baseline", uncapitalize(value)));
        }
        if let Some(value) = self.line_height {
            let unit = match value {
                LineHeight::Multiplier(number) => Some((number, "x")),
                LineHeight::Absolute(number) => Some((number, "px")),
                LineHeight::Normal => None,
            };
            if let Some((number, unit)) = unit {
                fields.push(DiagnosticsNode::property(
                    &DiagnosticsProperty::double("height", Some(f64::from(number))).unit(unit),
                ));
            } else {
                fields.push(message("height", "normal"));
            }
        }
        if let Some(value) = self.leading_distribution {
            fields.push(message("leadingDistribution", uncapitalize(value)));
        }
        if let Some(value) = &self.locale {
            fields.push(message("locale", value.clone()));
        }
        if let Some(value) = self.foreground {
            fields.push(color("foreground", value));
        }
        if let Some(value) = self.background {
            fields.push(color("background", value));
        }
        let mut decoration = Vec::new();
        if let Some(value) = self.decoration_style {
            decoration.push(uncapitalize(value));
        }
        if let Some(value) = self.decoration_color {
            let property = DiagnosticsProperty::color("decorationColor", Some(value))
                .level(DiagnosticLevel::Fine);
            decoration.push(property.to_description());
            fields.push(DiagnosticsNode::property(&property));
        }
        if let Some(value) = self.decoration {
            decoration.push(decoration_name(value));
        }
        if !decoration.is_empty() {
            fields.push(message("decoration", decoration.join(" ")));
        }
        if let Some(value) = self.decoration_thickness {
            fields.push(DiagnosticsNode::property(
                &DiagnosticsProperty::double("decorationThickness", Some(f64::from(value)))
                    .unit("x"),
            ));
        }
        let styled = !fields.is_empty();
        let mut result = Vec::with_capacity(fields.len() + 2);
        if let Some(value) = &self.debug_label {
            result.push(message("debugLabel", value.clone()));
        }
        result.push(DiagnosticsNode::property(
            &DiagnosticsProperty::new("inherit", Some(self.inherit)).level(
                if styled || !self.inherit {
                    DiagnosticLevel::Info
                } else {
                    DiagnosticLevel::Fine
                },
            ),
        ));
        result.append(&mut fields);
        if !styled {
            result.push(DiagnosticsNode::property(&DiagnosticsProperty::message(
                if self.inherit {
                    "<all styles inherited>"
                } else {
                    "<no style specified>"
                },
            )));
        }
        if let Some(value) = self.overflow {
            result.push(message("overflow", uncapitalize(value)));
        }
        result
    }
}

fn cursor_description(cursor: MouseCursor) -> Option<String> {
    let kind = match cursor {
        MouseCursor::Defer => return None,
        MouseCursor::Basic => "basic",
        MouseCursor::Click => "click",
        MouseCursor::Text => "text",
        MouseCursor::Crosshair => "precise",
        MouseCursor::Grab => "grab",
        MouseCursor::Grabbing => "grabbing",
        MouseCursor::NotAllowed => "forbidden",
        MouseCursor::ResizeHorizontal => "resizeLeftRight",
        MouseCursor::ResizeVertical => "resizeUpDown",
        MouseCursor::ResizeUpLeftDownRight => "resizeUpLeftDownRight",
        MouseCursor::ResizeUpRightDownLeft => "resizeUpRightDownLeft",
    };
    Some(format!("SystemMouseCursor({kind})"))
}

impl TextSpan {
    /// Snapshots this span with the visible paint style, literal text,
    /// pointer and semantic annotations, and its nested span nodes.
    #[must_use]
    pub fn to_diagnostics_node(&self) -> DiagnosticsNode {
        let mut properties = self
            .style
            .as_ref()
            .map(TextStyle::diagnostic_properties)
            .unwrap_or_default();
        if !self.text.is_empty() {
            properties.push(DiagnosticsNode::property(
                &DiagnosticsProperty::string("text", Some(self.text.clone())).show_name(false),
            ));
        } else if self.style.is_none() && self.children.is_empty() {
            properties.push(DiagnosticsNode::property(&DiagnosticsProperty::message(
                "(empty)",
            )));
        }
        if let Some(mouse) = &self.mouse {
            let callbacks = [
                (mouse.on_enter.is_some(), "enter"),
                (mouse.on_exit.is_some(), "exit"),
            ];
            let callbacks: Vec<_> = callbacks
                .into_iter()
                .filter_map(|(has, name)| has.then_some(name))
                .collect();
            if !callbacks.is_empty() {
                properties.push(message("callbacks", callbacks.join(", ")));
            }
            if let Some(cursor) = cursor_description(mouse.cursor) {
                properties.push(message("mouseCursor", cursor));
            }
        }
        for (name, value) in [
            ("semanticsLabel", &self.semantics_label),
            ("semanticsIdentifier", &self.semantics_identifier),
        ] {
            if let Some(value) = value {
                properties.push(DiagnosticsNode::property(&DiagnosticsProperty::string(
                    name,
                    Some(value.clone()),
                )));
            }
        }
        let children = self
            .children
            .iter()
            .map(|child| match child {
                InlineSpan::Text(child) => child.to_diagnostics_node(),
                InlineSpan::Widget(child) => widget_node(child),
            })
            .collect();
        DiagnosticsNode::new("TextSpan", "TextSpan")
            .style(DiagnosticsTreeStyle::Whitespace)
            .with_properties(properties)
            .with_children(children)
    }

    /// Formats the debug span tree using Flutter's whitespace style.
    #[must_use]
    pub fn to_string_deep(&self) -> String {
        if !cfg!(debug_assertions) {
            return String::new();
        }
        self.to_diagnostics_node()
            .to_string_deep(DiagnosticLevel::Debug)
    }
}

fn widget_node(span: &WidgetSpan) -> DiagnosticsNode {
    DiagnosticsNode::new("WidgetSpan", "WidgetSpan")
        .style(DiagnosticsTreeStyle::Whitespace)
        .with_properties(vec![
            message("alignment", uncapitalize(span.alignment)),
            message("child", format!("widget {}", span.id)),
        ])
}
