//! Flutter-style trace filtering and diagnostic console output.

use crate::{StackFrame, StackFrameFormat};

/// Origin of the trace, independent of where it is being displayed.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum StackFrameOrigin {
    /// Numbered VM and test-runner frames.
    #[default]
    Native,
    /// Source-mapped web frames.
    WebDebug,
    /// Compiled profile, release, and WebAssembly frames.
    WebNonDebug,
}

impl StackFrameOrigin {
    const fn frame_format(self) -> StackFrameFormat {
        match self {
            Self::Native | Self::WebDebug => StackFrameFormat::Debug,
            Self::WebNonDebug => StackFrameFormat::WebNonDebug,
        }
    }

    const fn is_web(self) -> bool {
        !matches!(self, Self::Native)
    }
}

/// Matches the assembled `scheme:package/path` of a frame.
#[derive(Clone, Copy)]
pub enum StackPackagePattern<'a> {
    /// A literal substring, including the empty substring.
    Contains(&'a str),
    /// A caller-owned predicate for patterns such as regular expressions.
    Predicate(&'a dyn Fn(&str) -> bool),
}

/// Selects a class/method inside a package path.
#[derive(Clone, Copy)]
pub struct StackFramePattern<'a> {
    pub package: StackPackagePattern<'a>,
    pub class_name: &'a str,
    pub method: &'a str,
}

impl StackFramePattern<'_> {
    /// Matches a canonical asynchronous suspension frame.
    pub const ASYNCHRONOUS_SUSPENSION: Self = Self {
        package: StackPackagePattern::Contains(""),
        class_name: "",
        method: "asynchronous suspension",
    };

    /// Checks the source-specific method spelling before matching the package.
    #[must_use]
    pub fn matches(&self, frame: &StackFrame<'_>, origin: StackFrameOrigin) -> bool {
        let method_matches = if origin.is_web() && self.method.starts_with('_') {
            frame
                .method
                .strip_prefix('[')
                .and_then(|method| method.strip_suffix(']'))
                == Some(self.method)
        } else {
            frame.method == self.method
        };
        if !method_matches || (!origin.is_web() && frame.class_name != self.class_name) {
            return false;
        }
        let qualified = format!(
            "{}:{}/{}",
            frame.package_scheme, frame.package, frame.package_path
        );
        match self.package {
            StackPackagePattern::Contains(needle) => qualified.contains(needle),
            StackPackagePattern::Predicate(predicate) => predicate(&qualified),
        }
    }
}

/// Sets a replacement reason for selected frames. Frames and reasons are
/// aligned after built-in package and class filtering.
pub trait StackFilter {
    fn filter<'a>(
        &'a self,
        frames: &[StackFrame<'_>],
        reasons: &mut [Option<&'a str>],
        origin: StackFrameOrigin,
    );
}

/// Replaces occurrences of a consecutive frame pattern with a single reason.
pub struct RepetitiveStackFrameFilter<'a> {
    pub frames: &'a [StackFramePattern<'a>],
    pub replacement: &'a str,
}

impl StackFilter for RepetitiveStackFrameFilter<'_> {
    fn filter<'a>(
        &'a self,
        frames: &[StackFrame<'_>],
        reasons: &mut [Option<&'a str>],
        origin: StackFrameOrigin,
    ) {
        let count = self.frames.len();
        if count == 0 {
            return;
        }
        // Flutter 3.50 stops just before the final otherwise valid start.
        let end = frames.len().saturating_sub(count);
        let mut index = 0;
        while index < end {
            let window = &frames[index..index + count];
            if self
                .frames
                .iter()
                .zip(window)
                .all(|(pattern, frame)| pattern.matches(frame, origin))
            {
                if let Some(reasons) = reasons.get_mut(index..index + count) {
                    reasons.fill(Some(self.replacement));
                }
                index += count;
            } else {
                index += 1;
            }
        }
    }
}

/// Formats an explicitly supplied trace, using application-provided filters
/// and an optional console sink rather than a process-wide Flutter singleton.
#[derive(Default)]
pub struct DiagnosticStack<'a> {
    pub origin: StackFrameOrigin,
    pub filters: &'a [&'a dyn StackFilter],
}

impl DiagnosticStack<'_> {
    /// Drops built-in internal frames, applies custom filters in order,
    /// and preserves the original text of all other recognized frames.
    #[must_use]
    pub fn default_stack_filter(&self, stack: &str) -> Vec<String> {
        let mut frames = StackFrame::parse_stack(stack, self.origin.frame_format());
        let mut skipped = 0;
        let mut sources = [false; ELISION_LABELS.len()];
        frames.retain(|frame| {
            let Some(index) = elision_index(frame) else {
                return true;
            };
            sources[index] = true;
            skipped += 1;
            false
        });

        let mut reasons = vec![None; frames.len()];
        for filter in self.filters {
            filter.filter(&frames, &mut reasons, self.origin);
        }

        let mut result = Vec::with_capacity(frames.len());
        let mut index = 0;
        while index < frames.len() {
            if let Some(reason) = reasons[index] {
                let start = index;
                while index + 1 < reasons.len() && reasons[index + 1] == Some(reason) {
                    index += 1;
                }
                if start == index {
                    result.push(format!("{reason} (1 frame)"));
                } else {
                    // Match the pinned user-facing count, including its
                    // extra increment for a multi-frame custom reason.
                    result.push(format!("{reason} ({} frames)", index - start + 2));
                }
            } else {
                result.push(frames[index].source.to_owned());
            }
            index += 1;
        }

        if skipped > 0 {
            let mut from: Vec<_> = ELISION_LABELS
                .iter()
                .zip(sources)
                .filter_map(|(label, included)| included.then_some(*label))
                .collect();
            from.sort_unstable();
            let summary = match from.as_slice() {
                [label] if skipped == 1 => format!("(elided one frame from {label})"),
                [label] => format!("(elided {skipped} frames from {label})"),
                [first, second] => format!("(elided {skipped} frames from {first} and {second})"),
                [prefix @ .., last] => {
                    format!(
                        "(elided {skipped} frames from {}, and {last})",
                        prefix.join(", ")
                    )
                }
                [] => unreachable!("a skipped frame always has a reason"),
            };
            result.push(summary);
        }
        result
    }

    /// Emits the label before the trace. The raw line limit is applied after
    /// removing web capture boilerplate but before dropping internal frames.
    /// Supply raw stack text from the diagnostic producer; native Rust
    /// backtrace capture is not implied by this API.
    pub fn debug_print_stack(
        &self,
        stack: &str,
        label: Option<&str>,
        max_frames: Option<usize>,
        mut emit: impl FnMut(&str),
    ) {
        if let Some(label) = label {
            emit(label);
        }
        let stack = stack
            .trim_end()
            .split('\n')
            .skip_while(|line| self.origin.is_web() && web_capture_frame(line))
            .take(max_frames.unwrap_or(usize::MAX))
            .collect::<Vec<_>>()
            .join("\n");
        emit(&self.default_stack_filter(&stack).join("\n"));
    }
}

const ELISION_LABELS: [&str; 8] = [
    "class _AssertionError",
    "class _FakeAsync",
    "class _FrameCallbackEntry",
    "class _Timer",
    "class _RawReceivePortImpl",
    "dart:async-patch",
    "dart:async",
    "package:stack_trace",
];

fn elision_index(frame: &StackFrame<'_>) -> Option<usize> {
    match frame.class_name {
        "_AssertionError" => Some(0),
        "_FakeAsync" => Some(1),
        "_FrameCallbackEntry" => Some(2),
        "_Timer" => Some(3),
        "_RawReceivePortImpl" => Some(4),
        _ => match (frame.package_scheme, frame.package) {
            ("dart", "async-patch") => Some(5),
            ("dart", "async") => Some(6),
            ("package", "stack_trace") => Some(7),
            _ => None,
        },
    }
}

fn web_capture_frame(line: &str) -> bool {
    [
        "StackTrace.current",
        "dart-sdk/lib/_internal",
        "dart:sdk_internal",
    ]
    .into_iter()
    .any(|text| line.contains(text))
}
