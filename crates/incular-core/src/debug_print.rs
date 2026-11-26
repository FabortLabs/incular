//! Console word wrapping and non-blocking, clocked debug log output.

use std::{collections::VecDeque, time::Duration};

use crate::time::Instant;

#[derive(Clone, Copy, Default, PartialEq, Eq)]
struct Cursor {
    byte: usize,
    utf16: usize,
}

impl Cursor {
    fn advance(&mut self, text: &str) {
        if let Some(next) = text[self.byte..].chars().next() {
            self.byte += next.len_utf8();
            self.utf16 += next.len_utf16();
        }
    }

    fn skip_spaces(&mut self, text: &str) {
        while text.as_bytes().get(self.byte) == Some(&b' ') {
            self.byte += 1;
            self.utf16 += 1;
        }
    }
}

/// Wraps one diagnostic line at ASCII spaces. Words aren't broken, and
/// stack-frame lines beginning with `#` are left alone. Width is measured
/// in UTF-16 units, as in the Flutter console API.
#[must_use]
pub fn debug_word_wrap(message: &str, width: usize, wrap_indent: &str) -> Vec<String> {
    let units = message.encode_utf16().count();
    let content = message.trim_start();
    if units < width || content.is_empty() || content.starts_with('#') {
        return vec![message.to_owned()];
    }

    let indentation = hanging_indent(message);
    let prefix_units = wrap_indent.encode_utf16().count() + indentation;
    let mut prefix = String::with_capacity(wrap_indent.len() + indentation);
    prefix.push_str(wrap_indent);
    prefix.extend(std::iter::repeat_n(' ', indentation));
    let mut start = Cursor::default();
    let mut end = Cursor::default();
    while end.byte < message.len() && end.utf16 < prefix_units {
        end.advance(message);
    }

    let mut result = Vec::new();
    let mut candidate = None;
    let mut prefix_width = 0;
    loop {
        end.skip_spaces(message);
        let word_start = end;
        while message
            .as_bytes()
            .get(end.byte)
            .is_some_and(|&byte| byte != b' ')
        {
            end.advance(message);
        }
        loop {
            let too_long = end.utf16 - start.utf16 + prefix_width > width;
            if !too_long && end.byte != message.len() {
                candidate = Some(end);
                break;
            }
            let chosen = if too_long {
                candidate.unwrap_or(end)
            } else {
                end
            };
            let mut line = String::new();
            if !result.is_empty() {
                line.push_str(&prefix);
            }
            line.push_str(&message[start.byte..chosen.byte]);
            result.push(line);

            if chosen.byte == message.len() {
                return result;
            }
            prefix_width = prefix_units;
            candidate = None;
            if chosen == end {
                end.skip_spaces(message);
                start = end;
                break;
            }
            // We broke before the current word; reconsider that word on
            // its own continuation line rather than scanning it twice.
            start = word_start;
        }
    }
}

/// Spaces in the leading indent and the width of a bullet or list marker.
fn hanging_indent(text: &str) -> usize {
    let bytes = text.as_bytes();
    let spaces = bytes.iter().take_while(|&&byte| byte == b' ').count();
    let tail = &bytes[spaces..];
    if tail.get(1) == Some(&b' ') && matches!(tail.first(), Some(b'-' | b'+' | b'*')) {
        return spaces + 2;
    }
    let digits = tail.iter().take_while(|byte| byte.is_ascii_digit()).count();
    if digits != 0
        && matches!(tail.get(digits), Some(b'.' | b')' | b':'))
        && tail.get(digits + 1) == Some(&b' ')
    {
        return spaces + digits + 2;
    }
    spaces
}

fn wrapped_lines(message: &str, width: usize) -> impl Iterator<Item = String> + '_ {
    message
        .split('\n')
        .flat_map(move |line| debug_word_wrap(line, width, ""))
}

/// Synchronously writes one message, joining wrapped lines before calling
/// the sink. An absent message is printed as the literal "null".
pub fn debug_print_synchronously(
    message: Option<&str>,
    wrap_width: Option<usize>,
    mut sink: impl FnMut(&str),
) {
    if let (Some(message), Some(width)) = (message, wrap_width) {
        let text = wrapped_lines(message, width).collect::<Vec<_>>().join("\n");
        sink(&text);
    } else {
        sink(message.unwrap_or("null"));
    }
}

/// Throttled debug output with a host-provided clock and sink.
///
/// Whole lines are emitted in order. If the budget is exhausted, the host
/// should arm one timer for [Self::next_deadline] and call [Self::flush_due]
/// at that time. No worker, sleep or frame polling is started by the buffer.
#[derive(Debug, Default)]
pub struct DebugPrintBuffer {
    queue: VecDeque<String>,
    charged: usize,
    idle_since: Option<Instant>,
    next_deadline: Option<Instant>,
}

impl DebugPrintBuffer {
    const CAPACITY: usize = 12 * 1024;
    const PAUSE: Duration = Duration::from_secs(1);

    /// Adds a message and emits any lines that are immediately eligible.
    pub fn print(
        &mut self,
        message: Option<&str>,
        wrap_width: Option<usize>,
        now: Instant,
        sink: impl FnMut(&str),
    ) {
        let message = message.unwrap_or("null");
        if let Some(width) = wrap_width {
            self.queue.extend(wrapped_lines(message, width));
        } else {
            self.queue.extend(message.split('\n').map(str::to_owned));
        }
        self.flush_due(now, sink);
    }

    /// Emits buffered lines when the budget permits. Calling before the
    /// current deadline does not spend more budget or move that deadline.
    pub fn flush_due(&mut self, now: Instant, mut sink: impl FnMut(&str)) {
        if self.next_deadline.is_some_and(|deadline| now < deadline) || self.queue.is_empty() {
            return;
        }
        self.next_deadline = None;
        if self
            .idle_since
            .is_some_and(|start| now.saturating_duration_since(start) > Self::PAUSE)
        {
            self.idle_since = None;
            self.charged = 0;
        }
        while self.charged < Self::CAPACITY {
            let Some(line) = self.queue.pop_front() else {
                break;
            };
            // The budget uses UTF-16 text units and never truncates a line.
            self.charged = self.charged.saturating_add(line.encode_utf16().count());
            sink(&line);
        }
        if self.queue.is_empty() {
            self.idle_since.get_or_insert(now);
        } else {
            self.charged = 0;
            self.next_deadline = now.checked_add(Self::PAUSE);
        }
    }

    #[must_use]
    pub const fn next_deadline(&self) -> Option<Instant> {
        self.next_deadline
    }

    #[must_use]
    pub fn is_drained(&self) -> bool {
        self.queue.is_empty()
    }
}
