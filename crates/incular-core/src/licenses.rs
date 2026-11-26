//! Design-system-neutral license notices and lazy collector streams.

use std::{
    cell::RefCell,
    pin::Pin,
    rc::Rc,
    sync::Arc,
    task::{Context, Poll},
};

use futures_core::Stream;

/// One license paragraph and its formatting indentation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LicenseParagraph {
    pub text: String,
    pub indent: i32,
}

impl LicenseParagraph {
    pub const CENTERED_INDENT: i32 = -1;

    #[must_use]
    pub fn new(text: impl Into<String>, indent: i32) -> Self {
        Self {
            text: text.into(),
            indent,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum LicenseText {
    HardWrapped(Arc<str>),
    Paragraphs(Arc<[LicenseParagraph]>),
}

/// A license that applies to one or more packages. Hard-wrapped source
/// text is retained verbatim and reflowed only when paragraphs are read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LicenseEntry {
    pub packages: Arc<[String]>,
    text: LicenseText,
}

impl LicenseEntry {
    /// Creates a hard-wrapped notice, as in `LicenseEntryWithLineBreaks`.
    #[must_use]
    pub fn with_line_breaks(
        packages: impl IntoIterator<Item = impl Into<String>>,
        text: impl Into<Arc<str>>,
    ) -> Self {
        Self {
            packages: packages.into_iter().map(Into::into).collect(),
            text: LicenseText::HardWrapped(text.into()),
        }
    }

    /// Creates an entry with explicit paragraphs and indentation.
    #[must_use]
    pub fn from_paragraphs(
        packages: impl IntoIterator<Item = impl Into<String>>,
        paragraphs: impl Into<Arc<[LicenseParagraph]>>,
    ) -> Self {
        Self {
            packages: packages.into_iter().map(Into::into).collect(),
            text: LicenseText::Paragraphs(paragraphs.into()),
        }
    }

    pub fn paragraphs(&self) -> impl Iterator<Item = LicenseParagraph> + '_ {
        match &self.text {
            LicenseText::HardWrapped(text) => Paragraphs::Parsed(LineParagraphs {
                remaining: text,
                previous_indent: 0,
            }),
            LicenseText::Paragraphs(paragraphs) => Paragraphs::Explicit(paragraphs.iter()),
        }
    }
}

enum Paragraphs<'a> {
    Parsed(LineParagraphs<'a>),
    Explicit(std::slice::Iter<'a, LicenseParagraph>),
}

impl Iterator for Paragraphs<'_> {
    type Item = LicenseParagraph;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Parsed(lines) => lines.next(),
            Self::Explicit(paragraphs) => paragraphs.next().cloned(),
        }
    }
}

struct LineParagraphs<'a> {
    remaining: &'a str,
    previous_indent: usize,
}

impl Iterator for LineParagraphs<'_> {
    type Item = LicenseParagraph;

    fn next(&mut self) -> Option<Self::Item> {
        let mut paragraph: Option<LicenseParagraph> = None;
        while !self.remaining.is_empty() {
            let mut origin = 0;
            let mut indent = 0_usize;
            let bytes = self.remaining.as_bytes();
            while let Some(&value) = bytes.get(origin) {
                match value {
                    b' ' => indent += 1,
                    b'\t' => indent += 8,
                    _ => break,
                }
                origin += 1;
            }
            let content = &self.remaining[origin..];
            if content.is_empty() {
                self.remaining = "";
                break;
            }
            // A blank line ends the current paragraph and resets its indent.
            let separator = match content.as_bytes()[0] {
                b'\r' if content.starts_with("\r\n") => 2,
                b'\r' | b'\n' | 0x0c => 1,
                _ => 0,
            };
            if separator != 0 {
                self.remaining = &content[separator..];
                self.previous_indent = 0;
                if paragraph.is_some() {
                    break;
                }
                continue;
            }
            // Square-bracket license preambles offset their first line
            // one extra space; the bracket remains in the visible text.
            indent += usize::from(content.starts_with('['));
            if paragraph.is_some() && indent > self.previous_indent {
                // Leave this line for the next paragraph, not the one we
                // are about to yield.
                break;
            }
            let end = content.find(['\n', '\x0c']).unwrap_or(content.len());
            let line = &content[..end];
            let line = line.strip_suffix('\r').unwrap_or(line);
            match &mut paragraph {
                Some(paragraph) => {
                    paragraph.text.push(' ');
                    paragraph.text.push_str(line);
                }
                None => {
                    let level = if indent > 10 {
                        LicenseParagraph::CENTERED_INDENT
                    } else {
                        (indent / 3) as i32
                    };
                    paragraph = Some(LicenseParagraph::new(line, level));
                }
            }
            self.previous_indent = indent;
            let Some(delimiter) = content.as_bytes().get(end) else {
                self.remaining = "";
                break;
            };
            self.remaining = &content[end + 1..];
            if *delimiter == 0x0c {
                self.previous_indent = 0;
                break;
            }
        }
        paragraph
    }
}

type EntryStream = Pin<Box<dyn Stream<Item = LicenseEntry>>>;
type Collector = Rc<dyn Fn() -> EntryStream>;

thread_local! {
    static COLLECTORS: RefCell<Vec<Collector>> = const { RefCell::new(Vec::new()) };
}

/// Per-UI-thread registry of package license collectors. Registration never
/// constructs a notice or starts a stream.
pub struct LicenseRegistry;

impl LicenseRegistry {
    pub fn add_license<S>(collector: impl Fn() -> S + 'static)
    where
        S: Stream<Item = LicenseEntry> + 'static,
    {
        COLLECTORS.with(|entries| {
            entries
                .borrow_mut()
                .push(Rc::new(move || Box::pin(collector())));
        });
    }

    /// Convenience registration for a collector that needs no async I/O.
    pub fn add_sync<I>(collector: impl Fn() -> I + 'static)
    where
        I: IntoIterator<Item = LicenseEntry>,
        I::IntoIter: Unpin + 'static,
    {
        Self::add_license(move || IterStream(collector().into_iter()));
    }

    /// Returns a fresh stream. Its collectors start on the first poll,
    /// in registration order, without starting later collectors early.
    #[must_use]
    pub fn licenses() -> LicenseStream {
        let empty = COLLECTORS.with(|all| all.borrow().is_empty());
        LicenseStream {
            pending: empty.then(|| Vec::new().into_iter()),
            current: None,
        }
    }

    /// Clears registrations for future reads, without invalidating streams
    /// that have already started iterating their snapshot.
    pub fn reset() {
        let previous = COLLECTORS.with(|entries| std::mem::take(&mut *entries.borrow_mut()));
        drop(previous);
    }
}

struct IterStream<I>(I);

impl<I: Iterator<Item = LicenseEntry> + Unpin> Stream for IterStream<I> {
    type Item = LicenseEntry;

    fn poll_next(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Poll::Ready(self.0.next())
    }
}

/// Lazily enumerates licenses from independent async collectors.
#[must_use = "license entries are collected only when the stream is polled"]
pub struct LicenseStream {
    pending: Option<std::vec::IntoIter<Collector>>,
    current: Option<EntryStream>,
}

impl Stream for LicenseStream {
    type Item = LicenseEntry;

    fn poll_next(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let stream = self.get_mut();
        let remaining = stream
            .pending
            .get_or_insert_with(|| COLLECTORS.with(|all| all.borrow().clone().into_iter()));
        loop {
            if let Some(current) = &mut stream.current {
                match current.as_mut().poll_next(context) {
                    Poll::Ready(None) => {}
                    result => return result,
                }
            }
            stream.current = remaining.next().map(|collector| collector());
            if stream.current.is_none() {
                return Poll::Ready(None);
            }
        }
    }
}
