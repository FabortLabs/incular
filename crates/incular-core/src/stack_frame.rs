//! Structured parsing of Flutter diagnostic stack text.

use std::{
    fmt,
    hash::{Hash, Hasher},
};

const UNKNOWN: &str = "<unknown>";
const ASYNC_GAP: &str = "===== asynchronous gap ===========================";

/// Interpretation of unnumbered frames. Numbered frames and gap markers are
/// accepted in both modes. Select the format of the originating runtime,
/// not the build configuration of the application consuming the trace.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum StackFrameFormat {
    /// Source-mapped Dart development compiler frames and test-runner frames.
    #[default]
    Debug,
    /// JavaScript profile/release frames and WebAssembly frames.
    WebNonDebug,
}

/// One Flutter stack trace frame. Frame number, line and column are `-1` when
/// unknown. Borrowing the input preserves the original text without copying it.
#[derive(Clone, Copy, Debug)]
pub struct StackFrame<'a> {
    pub number: i64,
    pub column: i64,
    pub line: i64,
    pub package_scheme: &'a str,
    pub package: &'a str,
    pub package_path: &'a str,
    pub class_name: &'a str,
    pub method: &'a str,
    pub is_constructor: bool,
    pub source: &'a str,
}

impl PartialEq for StackFrame<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.identity() == other.identity()
    }
}

impl Eq for StackFrame<'_> {}

impl Hash for StackFrame<'_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.identity().hash(state);
    }
}

impl StackFrame<'_> {
    /// Canonical frame for a VM async suspension or package stack trace gap.
    pub const ASYNCHRONOUS_SUSPENSION: Self =
        Self::marker("asynchronous suspension", "<asynchronous suspension>");
    /// Canonical frame for a VM trace abbreviated after an overflow.
    pub const STACK_OVERFLOW_ELISION: Self = Self::marker("...", "...");

    const fn marker(method: &'static str, source: &'static str) -> Self {
        Self {
            number: -1,
            column: -1,
            line: -1,
            package_scheme: "",
            package: "",
            package_path: "",
            class_name: "",
            method,
            is_constructor: false,
            source,
        }
    }

    // Flutter's equality and hash do not include the scheme, path or
    // constructor flag. Callers can still inspect those fields separately.
    fn identity(&self) -> (i64, &str, i64, i64, &str, &str, &str) {
        (
            self.number,
            self.package,
            self.line,
            self.column,
            self.class_name,
            self.method,
            self.source,
        )
    }
}

impl<'a> StackFrame<'a> {
    /// Parses a single line in debug mode without trimming it.
    #[must_use]
    pub fn from_stack_trace_line(line: &'a str) -> Option<Self> {
        Self::parse_line(line, StackFrameFormat::Debug)
    }

    /// Parses one line. Malformed input returns `None`; even malformed
    /// numbered input from an external diagnostic never panics.
    #[must_use]
    pub fn parse_line(line: &'a str, format: StackFrameFormat) -> Option<Self> {
        match line {
            "<asynchronous suspension>" | ASYNC_GAP => Some(Self::ASYNCHRONOUS_SUSPENSION),
            "..." => Some(Self::STACK_OVERFLOW_ELISION),
            _ if line.starts_with('#') => Self::numbered(line),
            _ => match format {
                StackFrameFormat::Debug => Self::web_debug(line),
                StackFrameFormat::WebNonDebug => Self::web_non_debug(line),
            },
        }
    }

    /// Parses a complete debug stack, skipping lines of unknown format.
    #[must_use]
    pub fn from_stack_string(stack: &'a str) -> Vec<Self> {
        Self::parse_stack(stack, StackFrameFormat::Debug)
    }

    /// Parses a complete stack with an explicit unnumbered-frame format.
    #[must_use]
    pub fn parse_stack(stack: &'a str, format: StackFrameFormat) -> Vec<Self> {
        stack
            .trim()
            .split('\n')
            .filter_map(|line| Self::parse_line(line, format))
            .collect()
    }

    fn numbered(source: &'a str) -> Option<Self> {
        let (number, tail) = source.strip_prefix('#')?.split_once(' ')?;
        let (name, location) = tail.trim_start_matches(' ').rsplit_once(" (")?;
        if name.is_empty() {
            return None;
        }
        let location = location.strip_suffix(')')?;
        let (package_scheme, path) = location
            .split_once(':')
            .filter(|(scheme, _)| is_uri_scheme(scheme))
            .unwrap_or(("", location));
        let path = uri_path(path);
        let (path, line, column) = match split_number_suffix(path) {
            Some((prefix, last)) => match split_number_suffix(prefix) {
                Some((path, line)) => (path, line, last),
                None => (prefix, last, -1),
            },
            None => (path, -1, -1),
        };
        let (package, package_path) = if matches!(package_scheme, "dart" | "package") {
            package_path_parts(path)
        } else {
            (UNKNOWN, path)
        };
        let (class_name, method, is_constructor) = if name.starts_with("new") {
            let qualified = name.split(' ').nth(1).unwrap_or(UNKNOWN);
            let (class, method) = class_and_method(qualified).unwrap_or((qualified, ""));
            (class, method, true)
        } else {
            let name = name.split(".<anonymous closure>").next().unwrap_or(name);
            let (class, method) = class_and_method(name).unwrap_or(("", name));
            (class, method, false)
        };
        Some(Self {
            number: digits(number)?,
            column,
            line,
            package_scheme,
            package,
            package_path,
            class_name,
            method,
            is_constructor,
            source,
        })
    }

    fn web_debug(source: &'a str) -> Option<Self> {
        // Get the rightmost " line:column " separator, as a source path
        // may itself contain spaces.
        for (end, char) in source.char_indices().rev() {
            if !char.is_whitespace() {
                continue;
            }
            let Some((path, entry)) = source[..end].rsplit_once(' ') else {
                continue;
            };
            let Some((line, column)) = entry.split_once(':') else {
                continue;
            };
            let (Some(line), Some(column)) = (digits(line), digits(column)) else {
                continue;
            };
            let method = source[end..].trim_start();
            if method.is_empty() {
                return None;
            }
            let (package_scheme, package, package_path) =
                if let Some(path) = path.strip_prefix("package:") {
                    let (package, path) = package_path_parts(uri_path(path));
                    ("package", package, path)
                } else {
                    (UNKNOWN, UNKNOWN, UNKNOWN)
                };
            return Some(Self {
                number: -1,
                column,
                line,
                package_scheme,
                package,
                package_path,
                class_name: UNKNOWN,
                method,
                is_constructor: false,
                source,
            });
        }
        None
    }

    fn web_non_debug(source: &'a str) -> Option<Self> {
        let name = source
            .trim_start()
            .strip_prefix("at ")?
            .split(char::is_whitespace)
            .next()
            .filter(|name| !name.is_empty())?;
        let (class_name, method) = name.split_once('.').unwrap_or((UNKNOWN, name));
        Some(Self {
            number: -1,
            column: -1,
            line: -1,
            package_scheme: UNKNOWN,
            package: UNKNOWN,
            package_path: UNKNOWN,
            class_name,
            method,
            is_constructor: false,
            source,
        })
    }
}

impl fmt::Display for StackFrame<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "StackFrame(#{}, {}:{}/{}:{}:{}, className: {}, method: {})",
            self.number,
            self.package_scheme,
            self.package,
            self.package_path,
            self.line,
            self.column,
            self.class_name,
            self.method
        )
    }
}

fn digits(value: &str) -> Option<i64> {
    if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    value.parse().ok()
}

fn split_number_suffix(value: &str) -> Option<(&str, i64)> {
    let (value, suffix) = value.rsplit_once(':')?;
    Some((value, digits(suffix)?))
}

fn is_uri_scheme(value: &str) -> bool {
    let mut bytes = value.bytes();
    bytes.next().is_some_and(|byte| byte.is_ascii_alphabetic())
        && bytes.all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'-' | b'.'))
}

fn uri_path(location: &str) -> &str {
    let location = location.split(['?', '#']).next().unwrap_or(location);
    if let Some(authority) = location.strip_prefix("//") {
        authority.find('/').map_or("", |start| &authority[start..])
    } else {
        location
    }
}

fn package_path_parts(path: &str) -> (&str, &str) {
    path.trim_start_matches('/')
        .split_once('/')
        .unwrap_or((path.trim_start_matches('/'), path))
}

fn class_and_method(qualified: &str) -> Option<(&str, &str)> {
    let (class, remainder) = qualified.split_once('.')?;
    Some((class, remainder.split('.').next().unwrap_or(remainder)))
}
