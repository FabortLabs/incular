//! Scalar diagnostic properties that retain their typed value in inspector JSON.

use std::fmt;

use serde_json::{Value, json};

/// Visibility of a diagnostic in ascending display order.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Ord, PartialOrd)]
pub enum DiagnosticLevel {
    Hidden,
    Fine,
    Debug,
    #[default]
    Info,
    Warning,
    Hint,
    Summary,
    Error,
    Off,
}

impl DiagnosticLevel {
    /// The portable inspector spelling.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Hidden => "hidden",
            Self::Fine => "fine",
            Self::Debug => "debug",
            Self::Info => "info",
            Self::Warning => "warning",
            Self::Hint => "hint",
            Self::Summary => "summary",
            Self::Error => "error",
            Self::Off => "off",
        }
    }
}

/// The tree presentation attached to a diagnostic property.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum DiagnosticsTreeStyle {
    /// Do not render a subtree.
    None,
    /// Use the normal sparse tree presentation.
    #[default]
    Sparse,
}

impl DiagnosticsTreeStyle {
    const fn name(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Sparse => "sparse",
        }
    }
}

/// A typed scalar value supported by the diagnostic property wire format.
pub trait DiagnosticValue: PartialEq {
    /// The scalar's logical type in diagnostics.
    const TYPE_NAME: &'static str;

    /// The underlying value, before presentation formatting.
    fn to_diagnostic_json(&self) -> Value;

    /// The undecorated scalar text.
    fn diagnostic_text(&self) -> String;

    /// A boolean value, when this type is a boolean.
    fn as_diagnostic_bool(&self) -> Option<bool> {
        None
    }
}

impl DiagnosticValue for String {
    const TYPE_NAME: &'static str = "String";
    fn diagnostic_text(&self) -> String {
        self.clone()
    }
    fn to_diagnostic_json(&self) -> Value {
        json!(self)
    }
}

impl DiagnosticValue for bool {
    const TYPE_NAME: &'static str = "bool";
    fn diagnostic_text(&self) -> String {
        self.to_string()
    }
    fn as_diagnostic_bool(&self) -> Option<bool> {
        Some(*self)
    }
    fn to_diagnostic_json(&self) -> Value {
        json!(self)
    }
}

impl DiagnosticValue for i64 {
    const TYPE_NAME: &'static str = "int";
    fn diagnostic_text(&self) -> String {
        self.to_string()
    }
    fn to_diagnostic_json(&self) -> Value {
        json!(self)
    }
}

impl DiagnosticValue for f64 {
    const TYPE_NAME: &'static str = "double";
    fn diagnostic_text(&self) -> String {
        if self.is_finite() {
            format!("{self:?}")
        } else {
            crate::debug_format_double(Some(*self))
        }
    }

    fn to_diagnostic_json(&self) -> Value {
        if self.is_finite() {
            json!(self)
        } else {
            json!(self.diagnostic_text())
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum Presentation<T> {
    Plain,
    String {
        quoted: bool,
    },
    Flag,
    Number {
        type_name: &'static str,
        format: fn(Option<&T>) -> String,
        unit_separator: &'static str,
    },
}

/// A named diagnostic with separately retained raw value and display text.
///
/// Presentation options cannot change the value serialized by
/// [Self::to_json]. A specified absent default differs from an unspecified
/// default so nullable diagnostics can be filtered correctly.
#[derive(Clone, Debug)]
pub struct DiagnosticsProperty<T> {
    name: String,
    value: Option<T>,
    default: Option<Option<T>>,
    description: Option<String>,
    if_empty: Option<String>,
    if_null: Option<String>,
    tooltip: Option<String>,
    unit: Option<String>,
    show_name: bool,
    level: DiagnosticLevel,
    missing_if_null: bool,
    style: DiagnosticsTreeStyle,
    flag_labels: [Option<String>; 2],
    presentation: Presentation<T>,
}

impl<T: DiagnosticValue> DiagnosticsProperty<T> {
    /// Constructs an ordinary scalar property.
    #[must_use]
    pub fn new(name: impl Into<String>, value: Option<T>) -> Self {
        Self {
            name: name.into(),
            value,
            default: None,
            description: None,
            if_empty: None,
            if_null: None,
            tooltip: None,
            unit: None,
            show_name: true,
            level: DiagnosticLevel::Info,
            missing_if_null: false,
            style: DiagnosticsTreeStyle::Sparse,
            flag_labels: [None, None],
            presentation: Presentation::Plain,
        }
    }

    /// Returns the original value, without formatting.
    #[must_use]
    pub fn value(&self) -> Option<&T> {
        self.value.as_ref()
    }

    /// Specifies the exact value that should lower the property's verbosity.
    #[must_use]
    pub fn default_value(mut self, value: Option<T>) -> Self {
        self.default = Some(value);
        self
    }

    /// Replaces the displayed description, not the underlying value.
    #[must_use]
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Supplies text for a null value.
    #[must_use]
    pub fn if_null(mut self, text: impl Into<String>) -> Self {
        self.if_null = Some(text.into());
        self
    }

    /// Supplies text for an empty value.
    #[must_use]
    pub fn if_empty(mut self, text: impl Into<String>) -> Self {
        self.if_empty = Some(text.into());
        self
    }

    /// Appends a parenthesized annotation to the displayed description.
    #[must_use]
    pub fn tooltip(mut self, text: impl Into<String>) -> Self {
        self.tooltip = Some(text.into());
        self
    }

    /// Labels a numeric property with a unit.
    #[must_use]
    pub fn unit(mut self, unit: impl Into<String>) -> Self {
        self.unit = Some(unit.into());
        self
    }

    /// Controls the rendered name prefix.
    #[must_use]
    pub fn show_name(mut self, show: bool) -> Self {
        self.show_name = show;
        self
    }

    /// Overrides the default visibility level.
    #[must_use]
    pub fn level(mut self, level: DiagnosticLevel) -> Self {
        self.level = level;
        self
    }

    /// Warns when the value was expected to be present.
    #[must_use]
    pub fn missing_if_null(mut self, missing: bool) -> Self {
        self.missing_if_null = missing;
        self
    }

    /// Sets the tree presentation carried by the property.
    #[must_use]
    pub fn style(mut self, style: DiagnosticsTreeStyle) -> Self {
        self.style = style;
        self
    }

    fn flag_label(&self) -> Option<&str> {
        let value = self.value.as_ref()?.as_diagnostic_bool()?;
        self.flag_labels[usize::from(value)].as_deref()
    }

    /// Whether the name is shown for the current value.
    #[must_use]
    pub fn shows_name(&self) -> bool {
        self.show_name
            || (matches!(self.presentation, Presentation::Flag) && self.flag_label().is_none())
    }

    /// Returns the effective level after default-value and missing-value rules.
    #[must_use]
    pub fn effective_level(&self) -> DiagnosticLevel {
        if self.level == DiagnosticLevel::Hidden
            || (matches!(self.presentation, Presentation::Flag)
                && self.value.is_some()
                && self.flag_label().is_none())
        {
            DiagnosticLevel::Hidden
        } else if self.value.is_none() && self.missing_if_null {
            DiagnosticLevel::Warning
        } else if self.default.as_ref() == Some(&self.value) {
            DiagnosticLevel::Fine
        } else {
            self.level
        }
    }

    /// Whether this property is below a requested visibility threshold.
    #[must_use]
    pub fn is_filtered(&self, minimum: DiagnosticLevel) -> bool {
        !cfg!(debug_assertions) || self.effective_level() < minimum
    }

    /// A display description without the property-name prefix.
    #[must_use]
    pub fn to_description(&self) -> String {
        let raw = |value: &T| {
            let text = value.diagnostic_text();
            let empty = self.if_empty.as_deref().filter(|_| text.is_empty());
            match self.presentation {
                Presentation::String { quoted: true } if empty.is_none() => format!("\"{text}\""),
                Presentation::Flag => self.flag_label().map_or(text, str::to_owned),
                Presentation::Number {
                    format,
                    unit_separator,
                    ..
                } => {
                    let number = format(Some(value));
                    match &self.unit {
                        Some(unit) => format!("{number}{unit_separator}{unit}"),
                        None => number,
                    }
                }
                _ => empty.map_or(text, str::to_owned),
            }
        };
        let text = self.description.clone().unwrap_or_else(|| {
            self.value.as_ref().map_or_else(
                || self.if_null.clone().unwrap_or_else(|| "null".to_owned()),
                raw,
            )
        });
        match &self.tooltip {
            Some(tooltip) => format!("{text} ({tooltip})"),
            None => text,
        }
    }

    /// Returns a scalar diagnostics wire object with typed JSON values.
    #[must_use]
    pub fn to_json(&self) -> Value {
        let default_type = format!("DiagnosticsProperty<{}>", T::TYPE_NAME);
        let type_name = match self.presentation {
            Presentation::Plain => &default_type,
            Presentation::String { .. } => "StringProperty",
            Presentation::Flag => "FlagProperty",
            Presentation::Number { type_name, .. } => type_name,
        };
        let level = self.effective_level();
        let mut output = json!({
            "description": self.to_description(),
            "type": type_name,
            "name": self.name,
            "value": self.value.as_ref().map(DiagnosticValue::to_diagnostic_json),
            "missingIfNull": self.missing_if_null,
            "propertyType": T::TYPE_NAME,
            "defaultLevel": self.level.name(),
        });
        if !self.shows_name() {
            output["showName"] = json!(false);
        }
        if self.style != DiagnosticsTreeStyle::Sparse {
            output["style"] = json!(self.style.name());
        }
        if level != DiagnosticLevel::Info {
            output["level"] = json!(level.name());
        }
        if let Some(default) = &self.default {
            output["defaultValue"] = json!(
                default
                    .as_ref()
                    .map_or_else(|| "null".to_owned(), DiagnosticValue::diagnostic_text)
            );
        }
        for (key, value) in [
            ("ifEmpty", &self.if_empty),
            ("ifNull", &self.if_null),
            ("tooltip", &self.tooltip),
        ] {
            if let Some(value) = value {
                output[key] = json!(value);
            }
        }
        match self.presentation {
            Presentation::String { quoted } => output["quoted"] = json!(quoted),
            Presentation::Flag => {
                for (key, label) in ["ifFalse", "ifTrue"].into_iter().zip(&self.flag_labels) {
                    if let Some(label) = label {
                        output[key] = json!(label);
                    }
                }
            }
            Presentation::Number { format, .. } => {
                output["numberToString"] = json!(format(self.value.as_ref()));
                if let Some(unit) = &self.unit {
                    output["unit"] = json!(unit);
                }
            }
            Presentation::Plain => {}
        }
        output
    }
}

impl DiagnosticsProperty<String> {
    /// Constructs a string property, double-quoted by default.
    #[must_use]
    pub fn string(name: impl Into<String>, value: Option<String>) -> Self {
        Self::new(name, value).quoted(true)
    }

    /// Controls whether a string property shows quotes around its value.
    #[must_use]
    pub fn quoted(mut self, quoted: bool) -> Self {
        self.presentation = Presentation::String { quoted };
        self
    }
}

impl DiagnosticsProperty<bool> {
    /// Constructs a boolean flag with at least one displayed state.
    ///
    /// A state without a configured label is hidden. If it is explicitly
    /// displayed, its label and raw value are shown as an ordinary property.
    #[must_use]
    pub fn flag(
        name: impl Into<String>,
        value: Option<bool>,
        if_true: Option<&str>,
        if_false: Option<&str>,
    ) -> Self {
        assert!(
            if_true.is_some() || if_false.is_some(),
            "a flag needs at least one label"
        );
        let mut property = Self::new(name, value).show_name(false).default_value(None);
        property.presentation = Presentation::Flag;
        property.flag_labels = [if_false.map(str::to_owned), if_true.map(str::to_owned)];
        property
    }
}

impl DiagnosticsProperty<i64> {
    /// Constructs an integer property with optional measurement units.
    #[must_use]
    pub fn integer(name: impl Into<String>, value: Option<i64>) -> Self {
        let mut property = Self::new(name, value);
        property.presentation = Presentation::Number {
            type_name: "IntProperty",
            unit_separator: "",
            format: |value| value.map_or_else(|| "null".to_owned(), ToString::to_string),
        };
        property
    }
}

impl DiagnosticsProperty<f64> {
    /// Constructs a floating-point property using diagnostic precision.
    #[must_use]
    pub fn double(name: impl Into<String>, value: Option<f64>) -> Self {
        let mut property = Self::new(name, value);
        property.presentation = Presentation::Number {
            type_name: "DoubleProperty",
            unit_separator: "",
            format: |value| crate::debug_format_double(value.copied()),
        };
        property
    }
    /// Describes a fraction from 0 to 1 as a percentage while retaining its
    /// original (possibly out-of-range) value.
    #[must_use]
    pub fn percent(name: impl Into<String>, value: Option<f64>) -> Self {
        let mut property = Self::new(name, value);
        property.presentation = Presentation::Number {
            type_name: "PercentProperty",
            unit_separator: " ",
            format: |value| {
                value.map_or_else(
                    || "null".to_owned(),
                    |value| {
                        let percent = crate::clamp_double(*value, 0., 1.) * 100.;
                        format!("{}%", crate::debug_double::format_double_fixed(percent, 1))
                    },
                )
            },
        };
        property
    }
}

impl<T: DiagnosticValue> fmt::Display for DiagnosticsProperty<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.shows_name() {
            write!(formatter, "{}: ", self.name)?;
        }
        formatter.write_str(&self.to_description())
    }
}
