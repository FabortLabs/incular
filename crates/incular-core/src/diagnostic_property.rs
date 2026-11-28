//! Scalar diagnostic properties that retain their typed value in inspector JSON.

use std::{
    cell::{OnceCell, RefCell},
    fmt,
    rc::Rc,
};

use indexmap::IndexMap;
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
    /// Render a tree with indentation but no connecting lines.
    Whitespace,
    /// Render a block without adding indentation.
    Flat,
    /// Frame an error report with a prominent heading and footer.
    Error,
    /// Place a diagnostic label above its value within an error report.
    ErrorProperty,
    /// Keep the named diagnostic and its value on one line.
    SingleLine,
}

impl DiagnosticsTreeStyle {
    pub(crate) const fn name(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Sparse => "sparse",
            Self::Whitespace => "whitespace",
            Self::Flat => "flat",
            Self::Error => "error",
            Self::ErrorProperty => "errorProperty",
            Self::SingleLine => "singleLine",
        }
    }
}

/// A typed scalar value supported by the diagnostic property wire format.
pub trait DiagnosticValue: PartialEq {
    /// The scalar's logical type in diagnostics.
    const TYPE_NAME: &'static str;

    /// The underlying value, before presentation formatting.
    fn to_diagnostic_json(&self) -> Value;

    /// Whether a present value is exposed as a JSON value at all.
    fn has_json_value(&self) -> bool {
        true
    }

    /// The undecorated scalar text.
    fn diagnostic_text(&self) -> String;

    /// A boolean value, when this type is a boolean.
    fn as_diagnostic_bool(&self) -> Option<bool> {
        None
    }
}

impl<T: DiagnosticValue> DiagnosticValue for Vec<T> {
    const TYPE_NAME: &'static str = "Iterable";

    fn to_diagnostic_json(&self) -> Value {
        Value::Array(
            self.iter()
                .map(DiagnosticValue::to_diagnostic_json)
                .collect(),
        )
    }

    fn has_json_value(&self) -> bool {
        false
    }

    fn diagnostic_text(&self) -> String {
        let elements: Vec<_> = self.iter().map(DiagnosticValue::diagnostic_text).collect();
        format!("[{}]", elements.join(", "))
    }
}

impl DiagnosticValue for () {
    const TYPE_NAME: &'static str = "void";

    fn to_diagnostic_json(&self) -> Value {
        Value::Null
    }

    fn diagnostic_text(&self) -> String {
        "null".to_owned()
    }
}

/// Ordered optional values for an inspector property that exposes only
/// their presence, never their actual objects.
#[derive(Clone, Debug, PartialEq)]
pub struct DiagnosticFlagEntries<T>(IndexMap<String, Option<T>>);

impl<T> DiagnosticFlagEntries<T> {
    /// Returns every submitted entry, including the absent ones.
    #[must_use]
    pub fn entries(&self) -> &IndexMap<String, Option<T>> {
        &self.0
    }

    fn present_names(&self) -> impl Iterator<Item = &str> {
        self.0
            .iter()
            .filter_map(|(name, value)| value.as_ref().map(|_| name.as_str()))
    }
}

impl<T: DiagnosticValue> DiagnosticValue for DiagnosticFlagEntries<T> {
    const TYPE_NAME: &'static str = "Map";

    fn to_diagnostic_json(&self) -> Value {
        Value::Null
    }

    fn has_json_value(&self) -> bool {
        false
    }

    fn diagnostic_text(&self) -> String {
        self.present_names().collect::<Vec<_>>().join(", ")
    }
}

/// An enum whose diagnostic label is the name of the selected member.
pub trait DiagnosticEnum: DiagnosticValue {
    /// Returns the short name, without the enum type prefix.
    fn variant_name(&self) -> &str;
}

/// Returns a typed enum member's short diagnostic name.
#[must_use]
pub fn describe_enum(value: &impl DiagnosticEnum) -> &str {
    value.variant_name()
}

/// An older enum-like description did not have a type/member separator.
#[derive(Debug, Eq, PartialEq, thiserror::Error)]
#[error("The provided object \"{0}\" is not an enum.")]
pub struct InvalidEnumDescription(String);

/// Extracts a short name from the text of a legacy enum-like object.
/// Real Rust enums should use [describe_enum].
pub fn describe_legacy_enum(display: &str) -> Result<&str, InvalidEnumDescription> {
    display
        .split_once('.')
        .filter(|(_, name)| !name.is_empty())
        .map(|(_, name)| name)
        .ok_or_else(|| InvalidEnumDescription(display.to_owned()))
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

impl DiagnosticValue for crate::Rect {
    const TYPE_NAME: &'static str = "Rect";

    fn diagnostic_text(&self) -> String {
        let right = self.origin.x + self.size.width;
        let bottom = self.origin.y + self.size.height;
        format!(
            "Rect.fromLTRB({:?}, {:?}, {right:?}, {bottom:?})",
            self.origin.x, self.origin.y
        )
    }

    fn to_diagnostic_json(&self) -> Value {
        Value::Null
    }

    fn has_json_value(&self) -> bool {
        false
    }
}

impl DiagnosticValue for crate::Color {
    const TYPE_NAME: &'static str = "Color";

    fn diagnostic_text(&self) -> String {
        let [a, r, g, b] = [self.alpha, self.red, self.green, self.blue].map(|component| {
            crate::debug_double::format_double_fixed(f64::from(component) / 255., 4)
        });
        format!("Color(alpha: {a}, red: {r}, green: {g}, blue: {b}, colorSpace: ColorSpace.sRGB)")
    }

    fn to_diagnostic_json(&self) -> Value {
        Value::Null
    }
    fn has_json_value(&self) -> bool {
        false
    }
}

/// A zero-argument callback whose identity is retained in diagnostics.
/// Formatting or serializing it never invokes the callback.
#[derive(Clone)]
pub struct DiagnosticCallback(Rc<dyn Fn()>);

impl DiagnosticCallback {
    /// Retains a callback for later application use or diagnostics.
    #[must_use]
    pub fn new(callback: impl Fn() + 'static) -> Self {
        Self(Rc::new(callback))
    }

    /// Invokes the application callback explicitly.
    pub fn invoke(&self) {
        (self.0)();
    }
}

impl PartialEq for DiagnosticCallback {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl fmt::Debug for DiagnosticCallback {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DiagnosticCallback")
            .finish_non_exhaustive()
    }
}

impl DiagnosticValue for DiagnosticCallback {
    const TYPE_NAME: &'static str = "Function";

    fn diagnostic_text(&self) -> String {
        "Closure: () => void".to_owned()
    }
    fn to_diagnostic_json(&self) -> Value {
        Value::Null
    }
    fn has_json_value(&self) -> bool {
        false
    }
}

/// A failed attempt to compute an inspector property.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiagnosticFailure {
    /// The human-facing error type.
    pub kind: String,
    /// The human-facing error message.
    pub message: String,
}

impl DiagnosticFailure {
    /// Describes a failed property computation without discarding its error.
    #[must_use]
    pub fn new(kind: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            kind: kind.into(),
            message: message.into(),
        }
    }
}

type ComputedValue<T> = Result<Option<T>, DiagnosticFailure>;
type DiagnosticComputation<T> = Box<dyn FnOnce() -> ComputedValue<T>>;

struct LazyValue<T> {
    compute: RefCell<Option<DiagnosticComputation<T>>>,
    cached: OnceCell<ComputedValue<T>>,
}

impl<T> LazyValue<T> {
    fn get(&self) -> Option<&ComputedValue<T>> {
        if self.cached.get().is_none() {
            // Take the closure and release the borrow before invoking it.
            // Reentrant observation sees an absent value until it finishes.
            let pending = self.compute.borrow_mut().take();
            if let Some(pending) = pending {
                let _ = self.cached.set(pending());
            }
        }
        self.cached.get()
    }
}

impl<T: fmt::Debug> fmt::Debug for LazyValue<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LazyDiagnosticValue")
            .field("cached", &self.cached.get())
            .finish()
    }
}

#[derive(Clone, Debug)]
enum PropertySource<T> {
    Eager(Option<T>),
    Lazy(Rc<LazyValue<T>>),
}

#[derive(Clone, Copy, Debug)]
enum Presentation<T> {
    Plain,
    String {
        quoted: bool,
    },
    Flag,
    ObjectFlag,
    Message,
    Color {
        channels: fn(&T) -> [u8; 4],
    },
    Iterable {
        member_type: &'static str,
        is_empty: fn(&T) -> bool,
        values_for_display: fn(&T) -> Vec<String>,
        values_for_json: fn(&T) -> Vec<String>,
    },
    FlagsSummary {
        member_type: &'static str,
        has_names: fn(&T) -> bool,
        names_for_json: fn(&T) -> Option<Vec<&str>>,
    },
    Enum {
        short_name: fn(&T) -> &str,
    },
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
    source: PropertySource<T>,
    default: Option<Option<T>>,
    description: Option<String>,
    if_empty: Option<String>,
    if_null: Option<String>,
    if_present: Option<String>,
    tooltip: Option<String>,
    unit: Option<String>,
    show_name: bool,
    show_separator: bool,
    level: DiagnosticLevel,
    missing_if_null: bool,
    allow_wrap: bool,
    allow_name_wrap: bool,
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
            source: PropertySource::Eager(value),
            default: None,
            description: None,
            if_empty: None,
            if_null: None,
            if_present: None,
            tooltip: None,
            unit: None,
            show_name: true,
            show_separator: true,
            level: DiagnosticLevel::Info,
            missing_if_null: false,
            allow_wrap: true,
            allow_name_wrap: true,
            style: DiagnosticsTreeStyle::Sparse,
            flag_labels: [None, None],
            presentation: Presentation::Plain,
        }
    }

    /// Describes the presence or absence of an opaque object, e.g. a callback.
    #[must_use]
    pub fn object_flag(
        name: impl Into<String>,
        value: Option<T>,
        if_present: Option<&str>,
        if_null: Option<&str>,
    ) -> Self {
        assert!(
            if_present.is_some() || if_null.is_some(),
            "an object flag needs at least one label"
        );
        let mut property = Self::new(name, value).show_name(false);
        property.if_present = if_present.map(str::to_owned);
        property.if_null = if_null.map(str::to_owned);
        property.presentation = Presentation::ObjectFlag;
        property
    }

    /// Displays `has {name}` when the named object is present.
    #[must_use]
    pub fn has(name: impl Into<String>, value: Option<T>) -> Self {
        let name = name.into();
        let label = format!("has {name}");
        Self::object_flag(name, value, Some(&label), None)
    }

    /// Computes a property only when it is first observed, retaining both
    /// successful values and failures across subsequent observation.
    #[must_use]
    pub fn lazy(
        name: impl Into<String>,
        compute: impl FnOnce() -> Result<Option<T>, DiagnosticFailure> + 'static,
    ) -> Self {
        let mut property = Self::new(name, None);
        property.source = PropertySource::Lazy(Rc::new(LazyValue {
            compute: RefCell::new(Some(Box::new(compute))),
            cached: OnceCell::new(),
        }));
        property
    }

    /// Returns the original value, without formatting.
    #[must_use]
    pub fn value(&self) -> Option<&T> {
        match &self.source {
            PropertySource::Eager(value) => value.as_ref(),
            PropertySource::Lazy(value) => value.get()?.as_ref().ok()?.as_ref(),
        }
    }

    /// Returns a failed computation, if this is a lazy property that failed.
    #[must_use]
    pub fn exception(&self) -> Option<&DiagnosticFailure> {
        match &self.source {
            PropertySource::Eager(_) => None,
            PropertySource::Lazy(value) => value.get()?.as_ref().err(),
        }
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

    /// Controls the colon between the displayed property name and value.
    #[must_use]
    pub fn show_separator(mut self, show: bool) -> Self {
        self.show_separator = show;
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
        let value = self.value()?.as_diagnostic_bool()?;
        self.flag_labels[usize::from(value)].as_deref()
    }

    fn object_flag_label(&self) -> Option<&str> {
        if self.value().is_some() {
            self.if_present.as_deref()
        } else {
            self.if_null.as_deref()
        }
    }

    /// Whether the name is shown for the current value.
    #[must_use]
    pub fn shows_name(&self) -> bool {
        self.show_name
            || match self.presentation {
                Presentation::Flag => self.flag_label().is_none(),
                Presentation::ObjectFlag => self.object_flag_label().is_none(),
                _ => false,
            }
    }

    /// Returns the effective level after default-value and missing-value rules.
    #[must_use]
    pub fn effective_level(&self) -> DiagnosticLevel {
        let hidden_flag = match self.presentation {
            Presentation::Flag => self.value().is_some() && self.flag_label().is_none(),
            Presentation::ObjectFlag => self.object_flag_label().is_none(),
            Presentation::FlagsSummary { has_names, .. } => {
                self.if_empty.is_none() && !self.value().is_some_and(has_names)
            }
            _ => false,
        };
        if self.level == DiagnosticLevel::Hidden || hidden_flag {
            DiagnosticLevel::Hidden
        } else if self.exception().is_some() {
            DiagnosticLevel::Error
        } else if self.value().is_none() && self.missing_if_null {
            DiagnosticLevel::Warning
        } else if matches!(self.presentation, Presentation::Iterable { is_empty, .. }
            if self.if_empty.is_none() && self.value().is_some_and(is_empty))
        {
            DiagnosticLevel::Fine
        } else if self
            .default
            .as_ref()
            .is_some_and(|default| default.as_ref() == self.value())
        {
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

    pub(crate) fn compact_iterable_description(&self) -> Option<String> {
        if let Presentation::Iterable {
            values_for_display, ..
        } = self.presentation
            && let Some(values) = self.value()
        {
            let members = values_for_display(values);
            if !members.is_empty() {
                return Some(format!("[{}]", members.join(", ")));
            }
        }
        None
    }

    /// Controls word wrapping of the description when rendering a tree.
    #[must_use]
    pub fn allow_wrap(mut self, allow: bool) -> Self {
        self.allow_wrap = allow;
        self
    }

    /// Controls word wrapping of the label independently of the description.
    #[must_use]
    pub fn allow_name_wrap(mut self, allow: bool) -> Self {
        self.allow_name_wrap = allow;
        self
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
                Presentation::ObjectFlag => self.if_present.as_deref().map_or(text, str::to_owned),
                Presentation::Enum { short_name } => short_name(value).to_owned(),
                Presentation::Iterable {
                    is_empty,
                    values_for_display,
                    ..
                } => {
                    if is_empty(value) {
                        self.if_empty.as_deref().unwrap_or("[]").to_owned()
                    } else {
                        let separator = match self.style {
                            DiagnosticsTreeStyle::None | DiagnosticsTreeStyle::SingleLine => ", ",
                            _ => "\n",
                        };
                        values_for_display(value).join(separator)
                    }
                }
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
        if self.description.is_none()
            && let Some(error) = self.exception()
        {
            return format!("EXCEPTION ({})", error.kind);
        }
        let text = self.description.clone().unwrap_or_else(|| {
            self.value().map_or_else(
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
        let generated_type;
        let specialized_property_type;
        let mut property_type = T::TYPE_NAME;
        let type_name = match self.presentation {
            Presentation::Plain => {
                generated_type = format!("DiagnosticsProperty<{}>", T::TYPE_NAME);
                generated_type.as_str()
            }
            Presentation::String { .. } => "StringProperty",
            Presentation::Flag => "FlagProperty",
            Presentation::Message => "MessageProperty",
            Presentation::Color { .. } => "ColorProperty",
            Presentation::Iterable { member_type, .. } => {
                generated_type = format!("IterableProperty<{member_type}>");
                specialized_property_type = format!("Iterable<{member_type}>");
                property_type = &specialized_property_type;
                generated_type.as_str()
            }
            Presentation::FlagsSummary { member_type, .. } => {
                generated_type = format!("FlagsSummary<{member_type}>");
                specialized_property_type = format!("Map<String, {member_type}?>");
                property_type = &specialized_property_type;
                generated_type.as_str()
            }
            Presentation::ObjectFlag => {
                generated_type = format!("ObjectFlagProperty<{}>", T::TYPE_NAME);
                generated_type.as_str()
            }
            Presentation::Enum { .. } => {
                generated_type = format!("EnumProperty<{}>", T::TYPE_NAME);
                generated_type.as_str()
            }
            Presentation::Number { type_name, .. } => type_name,
        };
        let level = self.effective_level();
        let mut output = json!({
            "description": self.to_description(),
            "type": type_name,
            "name": self.name,
            "missingIfNull": self.missing_if_null,
            "propertyType": property_type,
            "defaultLevel": self.level.name(),
        });
        match self.value() {
            Some(value) if value.has_json_value() => output["value"] = value.to_diagnostic_json(),
            None => output["value"] = Value::Null,
            _ => {}
        }
        if let Some(error) = self.exception() {
            output["exception"] = json!(error.message);
        }
        if !self.shows_name() {
            output["showName"] = json!(false);
        }
        if !self.show_separator {
            output["showSeparator"] = json!(false);
        }
        if !self.allow_wrap {
            output["allowWrap"] = json!(false);
        }
        if !self.allow_name_wrap {
            output["allowNameWrap"] = json!(false);
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
            ("ifPresent", &self.if_present),
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
                output["numberToString"] = json!(format(self.value()));
                if let Some(unit) = &self.unit {
                    output["unit"] = json!(unit);
                }
            }
            Presentation::Color { channels } => {
                if let Some([red, green, blue, alpha]) = self.value().map(channels) {
                    output["valueProperties"] = json!({
                        "red": red, "green": green, "blue": blue, "alpha": alpha,
                    });
                }
            }
            Presentation::Iterable {
                values_for_json, ..
            } => {
                if let Some(values) = self.value().map(values_for_json) {
                    output["values"] = json!(values);
                }
            }
            Presentation::FlagsSummary { names_for_json, .. } => {
                if let Some(names) = self.value().and_then(names_for_json) {
                    output["values"] = json!(names);
                }
            }
            Presentation::Plain
            | Presentation::ObjectFlag
            | Presentation::Message
            | Presentation::Enum { .. } => {}
        }
        output
    }
}

impl<T: DiagnosticValue> DiagnosticsProperty<Vec<T>> {
    /// Builds an IterableProperty, with separate member strings in the
    /// inspector representation and typed values available to the caller.
    #[must_use]
    pub fn iterable(name: impl Into<String>, value: Option<Vec<T>>) -> Self {
        let mut row = Self::new(name, value)
            .if_empty("[]")
            .style(DiagnosticsTreeStyle::SingleLine);
        row.presentation = Presentation::Iterable {
            member_type: T::TYPE_NAME,
            is_empty: Vec::is_empty,
            values_for_display: |values| {
                values
                    .iter()
                    .map(|value| {
                        if T::TYPE_NAME == "double"
                            && let Some(number) = value.to_diagnostic_json().as_f64()
                        {
                            crate::debug_format_double(Some(number))
                        } else {
                            value.diagnostic_text()
                        }
                    })
                    .collect()
            },
            values_for_json: |values| {
                values
                    .iter()
                    .map(DiagnosticValue::diagnostic_text)
                    .collect()
            },
        };
        row
    }
}

impl DiagnosticsProperty<()> {
    /// A diagnostic description with no label or concrete value.
    #[must_use]
    pub fn message(message: impl Into<String>) -> Self {
        Self::new("", None)
            .description(message)
            .show_name(false)
            .style(DiagnosticsTreeStyle::SingleLine)
    }

    /// A labeled diagnostic message with no concrete value.
    #[must_use]
    pub fn message_property(name: impl Into<String>, message: impl Into<String>) -> Self {
        let mut property = Self::new(name, None)
            .description(message)
            .style(DiagnosticsTreeStyle::SingleLine);
        property.presentation = Presentation::Message;
        property
    }
}

impl<T: DiagnosticValue> DiagnosticsProperty<DiagnosticFlagEntries<T>> {
    /// Summarizes optional values by showing the names whose values are
    /// present, in insertion order. No values are inspected or invoked.
    #[must_use]
    pub fn flags_summary(name: impl Into<String>, values: IndexMap<String, Option<T>>) -> Self {
        let mut property = Self::new(name, Some(DiagnosticFlagEntries(values)))
            .style(DiagnosticsTreeStyle::SingleLine);
        property.presentation = Presentation::FlagsSummary {
            member_type: T::TYPE_NAME,
            has_names: |flags| flags.present_names().next().is_some(),
            names_for_json: |flags| (!flags.0.is_empty()).then(|| flags.present_names().collect()),
        };
        property
    }
}

impl<T: DiagnosticEnum> DiagnosticsProperty<T> {
    /// Constructs an enum property that displays only the selected member.
    #[must_use]
    pub fn enumeration(name: impl Into<String>, value: Option<T>) -> Self {
        let mut property = Self::new(name, value);
        property.presentation = Presentation::Enum {
            short_name: T::variant_name,
        };
        property
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

impl DiagnosticsProperty<crate::Color> {
    /// Creates an inspector color property with 8-bit channels in
    /// `valueProperties` when present. Transparent black is still a value.
    #[must_use]
    pub fn color(name: impl Into<String>, value: Option<crate::Color>) -> Self {
        let mut property = Self::new(name, value);
        property.presentation = Presentation::Color {
            channels: |color| [color.red, color.green, color.blue, color.alpha],
        };
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
        Self::new(name, value).into_double_property()
    }

    /// A lazily resolved double, with the same display as an eager double.
    #[must_use]
    pub fn lazy_double(
        name: impl Into<String>,
        compute: impl FnOnce() -> Result<Option<f64>, DiagnosticFailure> + 'static,
    ) -> Self {
        Self::lazy(name, compute).into_double_property()
    }

    fn into_double_property(mut self) -> Self {
        self.presentation = Presentation::Number {
            type_name: "DoubleProperty",
            unit_separator: "",
            format: |value| crate::debug_format_double(value.copied()),
        };
        self
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
        let description = self.to_description();
        if self.shows_name() {
            let multiline = matches!(self.presentation, Presentation::Iterable { .. })
                && description.contains('\n');
            let separator = match (self.show_separator, multiline) {
                (true, true) => ":\n",
                (true, false) => ": ",
                (false, true) => "\n",
                (false, false) => " ",
            };
            write!(formatter, "{}{separator}", self.name)?;
        }
        formatter.write_str(&description)
    }
}
