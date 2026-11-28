use std::borrow::Cow;

use serde_json::Value;

use super::{DiagnosticLevel, DiagnosticsNode};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Style {
    Sparse,
    Dense,
    Offstage,
    Transition,
    Whitespace,
    Flat,
    Error,
    ErrorProperty,
    Inline,
}

impl Style {
    fn of(node: &DiagnosticsNode) -> Self {
        match node.text("style") {
            Some("dense") => Self::Dense,
            Some("offstage") => Self::Offstage,
            Some("transition") => Self::Transition,
            Some("whitespace") => Self::Whitespace,
            Some("flat") => Self::Flat,
            Some("error") => Self::Error,
            Some("errorProperty") => Self::ErrorProperty,
            Some("singleLine" | "none") => Self::Inline,
            _ => Self::Sparse,
        }
    }

    fn root_indent(self) -> &'static str {
        match self {
            Self::Sparse | Self::Offstage => " ",
            Self::Whitespace => "  ",
            _ => "",
        }
    }

    fn block_spacing(self) -> bool {
        matches!(self, Self::Sparse | Self::Offstage)
    }

    fn caption_colon(self) -> bool {
        matches!(self, Self::Transition | Self::Whitespace | Self::Flat)
    }

    fn body_indent(self) -> &'static str {
        if self == Self::Transition { "  " } else { "" }
    }

    fn child_link(self, last: bool) -> Link {
        let (first, rail, inside) = match (self, last) {
            (Self::Dense, false) => ("├", "│", ""),
            (Self::Dense, true) => ("└", " ", ""),
            (Self::Sparse, false) => ("├─", "│", " "),
            (Self::Sparse, true) => ("└─", " ", " "),
            (Self::Offstage, false) => ("╎╌", "╎", " "),
            (Self::Offstage, true) => ("└╌", " ", " "),
            (Self::Transition, false) => ("╞═╦══ ", "│", " ║ "),
            (Self::Transition, true) => ("╘═╦══ ", " ", " ║ "),
            (Self::Error, false) => ("╞═╦", "│", " ║ "),
            (Self::Error, true) => ("╘═╦", " ", " ║ "),
            (Self::Whitespace, _) => ("", " ", " "),
            _ => ("", "", ""),
        };
        Link {
            first,
            rail,
            inside,
        }
    }
}

struct Link {
    first: &'static str,
    rail: &'static str,
    inside: &'static str,
}

#[derive(Clone, Copy)]
struct Settings {
    minimum: DiagnosticLevel,
    wrap_width: usize,
    wrap_width_properties: usize,
}

fn columns(text: &str) -> usize {
    text.encode_utf16().count()
}

/// Wrap at ASCII spaces without breaking a token, an unwrappable span, or
/// the leading whitespace that is part of a line supplied by the caller.
fn wrap_line(
    text: &str,
    first_width: usize,
    next_width: usize,
    wrappable: impl Fn(usize) -> bool,
) -> Vec<&str> {
    let mut previous = None;
    let mut start = (0, 0);
    let mut width = first_width;
    let mut result = Vec::new();
    let mut column = 0;
    let mut inside_word = false;
    let mut inside_spaces = true;
    let boundaries = text.char_indices().filter_map(|(offset, character)| {
        let at = column;
        column += character.len_utf16();
        if character == ' ' {
            let boundary = inside_word && !inside_spaces && wrappable(offset);
            inside_spaces = true;
            boundary.then_some((offset, at))
        } else {
            inside_word = true;
            inside_spaces = false;
            None
        }
    });

    let next_word = |at: (usize, usize)| {
        let offset = at.0
            + text[at.0..]
                .bytes()
                .take_while(|byte| *byte == b' ')
                .count();
        (offset, at.1 + (offset - at.0))
    };
    for end in boundaries {
        if end.1.saturating_sub(start.1) > width {
            let split = previous.take().unwrap_or(end);
            result.push(&text[start.0..split.0]);
            start = next_word(split);
            width = next_width;
        }
        if end.0 >= start.0 {
            previous = Some(end);
        }
    }
    if column.saturating_sub(start.1) > width
        && let Some(split) = previous
        && split.0 > start.0
    {
        result.push(&text[start.0..split.0]);
        start = next_word(split);
    }
    if start.0 < text.len() || result.is_empty() {
        result.push(&text[start.0..]);
    }
    result
}

fn append_lines(output: &mut String, lines: &[&str], first: &str, continuing: &str) {
    for (index, line) in lines.iter().enumerate() {
        let prefix = if index == 0 { first } else { continuing };
        if index != 0 {
            output.push('\n');
        }
        if line.trim_end().is_empty() {
            output.push_str(prefix.trim_end());
        } else {
            output.push_str(prefix);
            output.push_str(line.trim_end());
        }
    }
}

fn repeat_border(output: &mut String, width: usize) {
    output.extend(std::iter::repeat_n('═', width));
}

impl std::fmt::Display for DiagnosticsNode {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let description = self.text("description").unwrap_or_default();
        if let Some(label) = self.label() {
            formatter.write_str(label)?;
            if !description.is_empty() {
                if self.flag("showSeparator", true) {
                    formatter.write_str(":")?;
                }
                formatter.write_str(if description.contains('\n') {
                    "\n"
                } else {
                    " "
                })?;
            }
        }
        formatter.write_str(description)
    }
}

impl DiagnosticsNode {
    /// Formats the visible diagnostic tree with the default text width.
    #[must_use]
    pub fn to_string_deep(&self, minimum: DiagnosticLevel) -> String {
        self.to_string_deep_with_widths(minimum, 65, 65)
    }

    /// Formats the diagnostic tree with a page width and a minimum width
    /// available to each property after indentation. Words are not split.
    #[must_use]
    pub fn to_string_deep_with_widths(
        &self,
        minimum: DiagnosticLevel,
        wrap_width: usize,
        wrap_width_properties: usize,
    ) -> String {
        if !cfg!(debug_assertions) {
            return String::new();
        }
        let settings = Settings {
            minimum,
            wrap_width,
            wrap_width_properties,
        };
        let mut output = String::new();
        self.render(
            &mut output,
            settings,
            "",
            Style::of(self).root_indent(),
            false,
        );
        output
    }

    fn text(&self, key: &str) -> Option<&str> {
        self.details.get(key).and_then(Value::as_str)
    }

    fn flag(&self, key: &str, default: bool) -> bool {
        self.details
            .get(key)
            .and_then(Value::as_bool)
            .unwrap_or(default)
    }

    fn label(&self) -> Option<&str> {
        self.text("name")
            .filter(|name| !name.is_empty() && self.flag("showName", true))
    }

    fn append_heading(
        &self,
        output: &mut String,
        first: &str,
        continuing: &str,
        description_prefix: &str,
        description: &str,
        width: Option<usize>,
    ) {
        let style = Style::of(self);
        let description = description.trim_end();
        let available =
            |prefix| width.map_or(usize::MAX, |width| width.saturating_sub(columns(prefix)));
        let allow_name_wrap = self.flag("allowNameWrap", false);
        let allow_wrap = self.flag("allowWrap", true);
        if let Some(name) = self.label() {
            let name = if style == Style::Error {
                Cow::Owned(name.to_uppercase())
            } else {
                Cow::Borrowed(name)
            };
            let separator = if !description.is_empty() && self.flag("showSeparator", true) {
                ":"
            } else {
                ""
            };
            let heading = format!("{name}{separator}");
            if description.is_empty()
                || description.contains('\n')
                || style == Style::ErrorProperty
                || (allow_name_wrap && columns(&heading) > available(first))
            {
                let lines = wrap_line(&heading, available(first), available(continuing), |_| {
                    allow_name_wrap
                });
                append_lines(output, &lines, first, continuing);
                if description.is_empty() {
                    return;
                }
                output.push('\n');
                for (index, line) in description.split('\n').enumerate() {
                    if index != 0 {
                        output.push('\n');
                    }
                    let lines = wrap_line(
                        line,
                        available(description_prefix),
                        available(description_prefix),
                        |_| allow_wrap,
                    );
                    append_lines(output, &lines, description_prefix, description_prefix);
                }
            } else {
                let label_length = heading.len();
                let joined = format!("{heading} {description}");
                let lines = wrap_line(
                    &joined,
                    available(first),
                    available(description_prefix),
                    |offset| {
                        if offset <= label_length {
                            allow_name_wrap
                        } else {
                            allow_wrap
                        }
                    },
                );
                append_lines(output, &lines, first, description_prefix);
            }
        } else {
            for (index, line) in description.split('\n').enumerate() {
                let prefix = if index == 0 { first } else { continuing };
                if index != 0 {
                    output.push('\n');
                }
                let lines = wrap_line(line, available(prefix), available(continuing), |_| {
                    allow_wrap
                });
                append_lines(output, &lines, prefix, continuing);
            }
        }
    }

    fn render_inline(
        &self,
        output: &mut String,
        minimum: DiagnosticLevel,
        as_property: bool,
        first: &str,
        continuing: &str,
        width: Option<usize>,
    ) {
        let description = if as_property {
            self.compact_description.as_deref()
        } else {
            None
        }
        .or_else(|| self.text("description"))
        .unwrap_or_default();
        let description_prefix = if self.label().is_some() {
            format!("{continuing}  ")
        } else {
            continuing.to_owned()
        };
        self.append_heading(
            output,
            first,
            continuing,
            &description_prefix,
            description,
            width,
        );
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
                property.render_inline(output, minimum, true, "", "  ", None);
            }
            output.push(')');
        }
    }

    fn render(
        &self,
        output: &mut String,
        settings: Settings,
        first: &str,
        continuing: &str,
        embedded: bool,
    ) {
        let style = Style::of(self);
        let width = settings
            .wrap_width
            .max(columns(continuing).saturating_add(settings.wrap_width_properties));
        if matches!(style, Style::Inline | Style::ErrorProperty) {
            let start = output.len();
            self.render_inline(
                output,
                settings.minimum,
                false,
                first,
                continuing,
                (embedded || style == Style::ErrorProperty).then_some(width),
            );
            if embedded || style == Style::ErrorProperty || output[start..].contains('\n') {
                output.push('\n');
            }
            return;
        }
        let properties: Vec<_> = self
            .properties
            .iter()
            .filter(|p| !p.filtered(settings.minimum))
            .collect();
        let connection = if style.block_spacing() {
            if self.children.is_empty() {
                "  "
            } else {
                "│ "
            }
        } else {
            ""
        };
        let caption_prefix = format!("{continuing}{connection}");
        let child_base = format!("{continuing}{}", style.body_indent());
        let body = format!("{caption_prefix}{}", style.body_indent());
        let description = self.text("description").unwrap_or_default();
        match style {
            Style::Dense => {
                self.render_inline(output, settings.minimum, false, first, continuing, None);
            }
            Style::Error => {
                self.append_heading(
                    output,
                    &format!("{first}══╡ "),
                    &body,
                    &body,
                    &description.to_uppercase(),
                    None,
                );
                output.push_str(" ╞══");
                let length = columns(output.rsplit('\n').next().unwrap_or_default());
                repeat_border(output, width.saturating_sub(length));
            }
            Style::Transition if self.label().is_some() => {
                let name = self.label().expect("transition has a label");
                output.push_str(first);
                output.push_str(name);
                if self.flag("showSeparator", true) {
                    output.push_str(" ═══");
                }
                output.push('\n');
                for (index, line) in description.trim_end().split('\n').enumerate() {
                    if index != 0 {
                        output.push('\n');
                    }
                    let available = width.saturating_sub(columns(&caption_prefix));
                    let lines = wrap_line(line, available, available, |_| {
                        self.flag("allowWrap", false)
                    });
                    append_lines(output, &lines, &caption_prefix, &caption_prefix);
                }
            }
            _ => self.append_heading(
                output,
                first,
                &caption_prefix,
                &caption_prefix,
                description,
                Some(width),
            ),
        }
        if style.caption_colon() && (!properties.is_empty() || !self.children.is_empty()) {
            output.push(':');
        }
        output.push('\n');

        if style != Style::Dense {
            for property in &properties {
                let link = Style::of(property).child_link(false);
                let hanging = Style::of(property).child_link(true);
                property.render(
                    output,
                    settings,
                    &format!("{body}{}", link.first),
                    &format!("{body}{}{}", hanging.rail, hanging.inside),
                    true,
                );
            }
        }
        if style.block_spacing()
            && self.children.is_empty()
            && !properties.is_empty()
            && !body.trim_end().is_empty()
        {
            output.push_str(body.trim_end());
            output.push('\n');
        }
        for (index, child) in self.children.iter().enumerate() {
            let last = index + 1 == self.children.len();
            let child_style = match Style::of(child) {
                Style::Inline | Style::ErrorProperty => style,
                other => other,
            };
            let link = child_style.child_link(last);
            if index == 0
                && !properties.is_empty()
                && style.block_spacing()
                && child_style.block_spacing()
            {
                output.push_str(body.trim_end());
                output.push('\n');
            }
            let child_continuing = format!("{child_base}{}{}", link.rail, link.inside);
            child.render(
                output,
                settings,
                &format!("{child_base}{}", link.first),
                &child_continuing,
                true,
            );
            if matches!(child_style, Style::Transition | Style::Error) {
                let footer = format!("{child_base}{} ╚", link.rail);
                let count = if child_style == Style::Error {
                    width
                        .max(
                            columns(&child_continuing)
                                .saturating_add(settings.wrap_width_properties),
                        )
                        .saturating_sub(columns(&footer))
                        .max(16)
                } else {
                    11
                };
                output.push_str(&footer);
                repeat_border(output, count);
                output.push('\n');
            }
        }
        if style == Style::Error && !embedded {
            output.push_str(continuing);
            repeat_border(output, width.saturating_sub(columns(continuing)).max(5));
            output.push('\n');
        }
    }
}
