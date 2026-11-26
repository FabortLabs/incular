use std::{
    cell::{Cell, RefCell},
    pin::Pin,
    rc::Rc,
    task::{Context, Poll, Waker},
};

use futures_core::Stream;
use incular_core::{LicenseEntry, LicenseParagraph, LicenseRegistry};

fn license(text: &str) -> LicenseEntry {
    LicenseEntry::with_line_breaks(Vec::<String>::new(), text)
}

fn paragraphs(text: &str) -> Vec<LicenseParagraph> {
    license(text).paragraphs().collect()
}

#[test]
fn hard_wrapped_text_reflows_with_flutter_indents_and_page_breaks() {
    let source = concat!(
        "A\nA\nA\n  B\nB\nB\n   C\nC\nC\n                  D\nD\nD\n\n",
        "E\nE\n F\n  G\n G\nG\n\n[H\n H\n H]\n  \nI\x0cJ\n\x0cK\nK\n\x0c\n",
        "L\nL L\nL  L\nL   L\nL    L\nL     L\n\n   M\nM   \nM    \n\n",
        "N\n\nO\nO\n\n\nP\n\n\n\nQQQ\n\nRR RRR RRRR RRRRR\nR\n\nS\n\n   T\n\n",
        "      U\n         V\n\n        W\n\n       X\n      \n      Y"
    );
    let expected = [
        ("A A A", 0),
        ("B B B", 0),
        ("C C C", 1),
        ("D D D", -1),
        ("E E", 0),
        ("F", 0),
        ("G G G", 0),
        ("[H H H]", 0),
        ("I", 0),
        ("J", 0),
        ("K K", 0),
        ("L L L L  L L   L L    L L     L", 0),
        ("M M    M    ", 1),
        ("N", 0),
        ("O O", 0),
        ("P", 0),
        ("QQQ", 0),
        ("RR RRR RRRR RRRRR R", 0),
        ("S", 0),
        ("T", 1),
        ("U", 2),
        ("V", 3),
        ("W", 2),
        ("X", 2),
        ("Y", 2),
    ];
    assert_eq!(
        paragraphs(source),
        expected.map(|(text, indent)| LicenseParagraph::new(text, indent))
    );
}

#[test]
fn leading_and_trailing_blank_lines_do_not_create_empty_paragraphs() {
    for whitespace in ["    \n\n    ", "    \r\n\r\n    ", "\t\x0c\r\n  "] {
        assert!(paragraphs(whitespace).is_empty());
    }
    for padded in ["    \nA\n    ", "\n\n\nA\n\n\n"] {
        assert_eq!(paragraphs(padded), [LicenseParagraph::new("A", 0)]);
    }
}

#[test]
fn carriage_return_line_endings_do_not_enter_reflowed_text() {
    assert_eq!(
        paragraphs("A\r\nB\r\nC\r\n\r\nD\r\nE"),
        [
            LicenseParagraph::new("A B C", 0),
            LicenseParagraph::new("D E", 0)
        ]
    );
}

fn poll<S: Stream + Unpin>(stream: &mut S) -> Poll<Option<S::Item>> {
    Pin::new(stream).poll_next(&mut Context::from_waker(Waker::noop()))
}

fn drain(mut stream: impl Stream<Item = LicenseEntry> + Unpin) -> Vec<String> {
    let mut got = Vec::new();
    loop {
        match poll(&mut stream) {
            Poll::Ready(Some(entry)) => got.push(entry.paragraphs().next().unwrap().text),
            Poll::Ready(None) => return got,
            Poll::Pending => panic!("test stream was expected to be ready"),
        }
    }
}

#[test]
fn collectors_are_lazy_and_repeat_in_registration_order() {
    LicenseRegistry::reset();
    assert!(drain(LicenseRegistry::licenses()).is_empty());
    let created_empty = LicenseRegistry::licenses();
    let launches = Rc::new(RefCell::new(Vec::new()));
    for [a, b] in [["A", "B"], ["C", "D"]] {
        let launches = launches.clone();
        LicenseRegistry::add_sync(move || {
            launches.borrow_mut().push(a);
            [license(a), license(b)]
        });
    }
    let unstarted = LicenseRegistry::licenses();
    assert!(launches.borrow().is_empty());
    assert!(drain(created_empty).is_empty());
    assert!(launches.borrow().is_empty());
    assert_eq!(drain(unstarted), ["A", "B", "C", "D"]);
    assert_eq!(drain(LicenseRegistry::licenses()), ["A", "B", "C", "D"]);
    assert_eq!(*launches.borrow(), ["A", "C", "A", "C"]);
}

struct WaitThenStream {
    step: usize,
}

impl Stream for WaitThenStream {
    type Item = LicenseEntry;

    fn poll_next(
        mut self: Pin<&mut Self>,
        context: &mut Context<'_>,
    ) -> Poll<Option<LicenseEntry>> {
        let step = self.step;
        self.step += 1;
        match step {
            0 => {
                context.waker().wake_by_ref();
                Poll::Pending
            }
            1 => Poll::Ready(Some(license("first"))),
            _ => Poll::Ready(None),
        }
    }
}

#[test]
fn a_pending_collector_does_not_start_the_next_one_and_reset_only_changes_future_reads() {
    LicenseRegistry::reset();
    let later_started = Rc::new(Cell::new(false));
    LicenseRegistry::add_license(|| WaitThenStream { step: 0 });
    LicenseRegistry::add_sync({
        let later_started = later_started.clone();
        move || {
            later_started.set(true);
            [license("second")]
        }
    });
    let mut stream = LicenseRegistry::licenses();
    assert!(poll(&mut stream).is_pending());
    assert!(!later_started.get());
    let Poll::Ready(Some(first)) = poll(&mut stream) else {
        panic!("first collector did not resume")
    };
    assert_eq!(first.paragraphs().next().unwrap().text, "first");
    assert!(!later_started.get());
    LicenseRegistry::reset();
    assert_eq!(drain(stream), ["second"]);
    assert!(later_started.get());
    assert!(drain(LicenseRegistry::licenses()).is_empty());
}

#[test]
fn custom_entry_paragraphs_and_utf8_source_are_both_retained() {
    let explicit = LicenseEntry::from_paragraphs(
        ["demo"],
        vec![LicenseParagraph::new(
            "Juridique 日本語",
            LicenseParagraph::CENTERED_INDENT,
        )],
    );
    assert_eq!(explicit.packages.as_ref(), ["demo"]);
    assert_eq!(
        explicit.paragraphs().next().unwrap().text,
        "Juridique 日本語"
    );
    assert_eq!(
        paragraphs("© demo\n日本語\n\tlegal\n  continuation"),
        [
            LicenseParagraph::new("© demo 日本語", 0),
            LicenseParagraph::new("legal continuation", 2)
        ]
    );
}
