use serde_json::Value;

use super::{DiagnosticLevel, DiagnosticsNode};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Style {
    Sparse,
    Whitespace,
    Flat,
    Inline,
}

impl Style {
    fn of(node: &DiagnosticsNode) -> Self {
        match node.text("style") {
            Some("sparse") => Self::Sparse,
            Some("whitespace") => Self::Whitespace,
            Some("flat") => Self::Flat,
            Some("singleLine" | "none") => Self::Inline,
            _ if node.specialized.contains_key("propertyType") => Self::Inline,
            _ => Self::Sparse,
        }
    }

    fn root_indent(self) -> &'static str {
        match self {
            Self::Sparse => " ",
            Self::Whitespace => "  ",
            Self::Flat | Self::Inline => "",
        }
    }
}

impl DiagnosticsNode {
    /// Formats the visible diagnostic tree, including its properties and children.
    #[must_use]
    pub fn to_string_deep(&self, minimum: DiagnosticLevel) -> String {
        if !cfg!(debug_assertions) {
            return String::new();
        }
        let mut output = String::new();
        self.render(
            &mut output,
            minimum,
            "",
            Style::of(self).root_indent(),
            false,
        );
        output
    }

    fn text(&self, key: &str) -> Option<&str> {
        self.details.get(key).and_then(Value::as_str)
    }

    fn label(&self) -> Option<&str> {
        self.text("name").filter(|name| {
            !name.is_empty() && self.details.get("showName").and_then(Value::as_bool) != Some(false)
        })
    }

    fn append_heading(
        &self,
        output: &mut String,
        prefix: &str,
        continuing: &str,
        description: &str,
    ) {
        output.push_str(prefix);
        if let Some(name) = self.label() {
            output.push_str(name);
            if !description.is_empty() {
                if self.details.get("showSeparator").and_then(Value::as_bool) != Some(false) {
                    output.push(':');
                }
                if description.contains('\n') {
                    output.push('\n');
                    output.push_str(continuing);
                } else {
                    output.push(' ');
                }
            }
        }
        for (index, line) in description.trim_end().split('\n').enumerate() {
            if index != 0 {
                output.push('\n');
                if !line.is_empty() {
                    output.push_str(continuing);
                }
            }
            output.push_str(line);
        }
    }

    fn render_inline(&self, output: &mut String, minimum: DiagnosticLevel, as_property: bool) {
        let description = if as_property {
            self.compact_description.as_deref()
        } else {
            None
        }
        .or_else(|| self.text("description"))
        .unwrap_or_default();
        self.append_heading(output, "", "  ", description);

        let mut properties = self
            .properties
            .iter()
            .filter(|p| !p.filtered(minimum))
            .peekable();
        if properties.peek().is_some() {
            output.push('(');
            for (index, property) in properties.enumerate() {
                if index != 0 {
                    output.push_str(", ");
                }
                property.render_inline(output, minimum, true);
            }
            output.push(')');
        }
    }

    fn render(
        &self,
        output: &mut String,
        minimum: DiagnosticLevel,
        first: &str,
        continuing: &str,
        embedded: bool,
    ) {
        let style = Style::of(self);
        if style == Style::Inline {
            output.push_str(first);
            let start = output.len();
            self.render_inline(output, minimum, false);
            if embedded || output[start..].contains('\n') {
                output.push('\n');
            }
            return;
        }
        let properties: Vec<_> = self
            .properties
            .iter()
            .filter(|p| !p.filtered(minimum))
            .collect();
        let connection = if style == Style::Sparse {
            if self.children.is_empty() {
                "  "
            } else {
                "│ "
            }
        } else {
            ""
        };
        let body = format!("{continuing}{connection}");
        self.append_heading(
            output,
            first,
            &body,
            self.text("description").unwrap_or_default(),
        );
        if matches!(style, Style::Whitespace | Style::Flat)
            && (!properties.is_empty() || !self.children.is_empty())
        {
            output.push(':');
        }
        output.push('\n');

        for property in &properties {
            property.render(
                output,
                minimum,
                &body,
                &format!("{body}{}", Style::of(property).root_indent()),
                true,
            );
        }
        for (index, child) in self.children.iter().enumerate() {
            let last = index + 1 == self.children.len();
            let child_style = match Style::of(child) {
                Style::Inline => style,
                other => other,
            };
            let (link, next) = match (child_style, last) {
                (Style::Sparse, true) => ("└─", "  "),
                (Style::Sparse, false) => ("├─", "│ "),
                (Style::Whitespace, _) => ("", "  "),
                _ => ("", ""),
            };
            if index == 0
                && !properties.is_empty()
                && style == Style::Sparse
                && child_style == Style::Sparse
            {
                output.push_str(continuing);
                output.push_str("│\n");
            }
            child.render(
                output,
                minimum,
                &format!("{continuing}{link}"),
                &format!("{continuing}{next}"),
                true,
            );
        }
    }
}
