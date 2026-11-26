use std::time::Duration;

use incular_core::{DebugPrintBuffer, debug_print_synchronously, debug_word_wrap, time::Instant};

#[test]
fn synchronous_and_throttled_wrapping_keep_the_original_sink_boundaries() {
    let mut log = Vec::new();
    debug_print_synchronously(Some("Hello, world"), None, |message| {
        log.push(message.to_owned())
    });
    assert_eq!(std::mem::take(&mut log), ["Hello, world"]);
    debug_print_synchronously(Some("Hello, world"), Some(10), |message| {
        log.push(message.to_owned())
    });
    assert_eq!(std::mem::take(&mut log), ["Hello,\nworld"]);

    for width in 0..14 {
        debug_print_synchronously(Some("Hello,   world"), Some(width), |message| {
            log.push(message.to_owned())
        });
        assert_eq!(std::mem::take(&mut log), ["Hello,\nworld"]);
    }

    let mut printer = DebugPrintBuffer::default();
    let now = Instant::now();
    printer.print(Some("Hello, world"), None, now, |message| {
        log.push(message.to_owned())
    });
    assert_eq!(std::mem::take(&mut log), ["Hello, world"]);
    printer.print(Some("Hello, world"), Some(10), now, |message| {
        log.push(message.to_owned())
    });
    assert_eq!(std::mem::take(&mut log), ["Hello,", "world"]);
    assert!(printer.next_deadline().is_none());
}

#[test]
fn throttling_defers_whole_lines_and_arms_exactly_one_deadline() {
    let epoch = Instant::now();
    let mut printer = DebugPrintBuffer::default();
    let mut seen = Vec::new();
    let mut sink = |value: &str| seen.push(value.to_owned());
    let oversized = "A".repeat(22 * 1024);
    printer.print(Some(&format!("{oversized}\nB")), None, epoch, &mut sink);
    assert_eq!(
        printer.next_deadline(),
        Some(epoch + Duration::from_secs(1))
    );
    assert!(!printer.is_drained());
    printer.flush_due(epoch + Duration::from_millis(999), &mut sink);
    printer.flush_due(epoch + Duration::from_secs(2), &mut sink);
    assert!(printer.is_drained());
    assert!(printer.next_deadline().is_none());
    assert_eq!(&seen, &[oversized, "B".into()]);

    let mut seen = Vec::new();
    let mut sink = |value: &str| seen.push(value.to_owned());
    let oversized = "C".repeat(22 * 1024);
    let next = epoch + Duration::from_secs(2);
    printer.print(Some(&oversized), None, next, &mut sink);
    assert!(printer.is_drained());
    printer.print(Some("D"), None, next, &mut sink);
    assert_eq!(printer.next_deadline(), Some(next + Duration::from_secs(1)));
    printer.flush_due(next + Duration::from_secs(2), &mut sink);
    assert_eq!(&seen, &[oversized, "D".into()]);
    assert!(printer.is_drained());
}

#[test]
fn a_null_message_prints_the_literal_with_or_without_wrapping() {
    let mut printer = DebugPrintBuffer::default();
    let mut seen = Vec::new();
    let now = Instant::now();
    for width in [None, Some(80)] {
        printer.print(None, width, now, |line| seen.push(line.to_owned()));
    }
    assert_eq!(seen, ["null", "null"]);
}

#[test]
fn continued_list_items_are_indented_and_stack_traces_are_not_broken() {
    assert_eq!(
        debug_word_wrap("  2. Alpha beta gamma", 13, ""),
        ["  2. Alpha", "     beta", "     gamma"],
    );
    assert_eq!(
        debug_word_wrap("    #12 file.dart:20", 2, ""),
        ["    #12 file.dart:20"]
    );
    assert_eq!(
        debug_word_wrap("foo extremelylongword", 3, ""),
        ["foo", "extremelylongword"]
    );
    assert_eq!(debug_word_wrap("😀 😀", 4, ""), ["😀", "😀"]);
    assert_eq!(debug_word_wrap("", 0, ""), [""]);
}

#[test]
fn budget_charges_utf16_and_coalesces_messages_until_the_same_wake() {
    let epoch = Instant::now();
    let mut printer = DebugPrintBuffer::default();
    let queued = "😀".repeat(6 * 1024);
    let mut sizes = Vec::new();
    printer.print(Some(&queued), None, epoch, |line| {
        sizes.push(line.encode_utf16().count())
    });
    assert!(printer.is_drained());
    printer.print(Some("X"), None, epoch + Duration::from_millis(2), |line| {
        sizes.push(line.len())
    });
    let deadline = printer.next_deadline().unwrap();
    printer.print(Some("Y"), None, epoch + Duration::from_millis(5), |line| {
        sizes.push(line.len())
    });
    assert_eq!(printer.next_deadline(), Some(deadline));
    assert_eq!(sizes, [12 * 1024]);
    printer.flush_due(deadline, |line| sizes.push(line.len()));
    assert_eq!(sizes, [12 * 1024, 1, 1]);
    assert!(printer.is_drained());
}
