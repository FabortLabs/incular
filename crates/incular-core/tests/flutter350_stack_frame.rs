use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher as _},
};

use incular_core::{StackFrame, StackFrameFormat};

const FIXTURE: &str = include_str!("fixtures/flutter350_stack_frames.tsv");
const ASYNC_GAP: &str = "===== asynchronous gap ===========================";

fn fixture(group: &str) -> Vec<StackFrame<'static>> {
    FIXTURE
        .lines()
        .skip_while(|line| line.strip_prefix('@') != Some(group))
        .skip(1)
        .take_while(|line| !line.starts_with('@'))
        .filter(|line| !line.is_empty())
        .map(|line| {
            let fields: Vec<_> = line.split('\t').collect();
            assert_eq!(fields.len(), 10);
            StackFrame {
                number: fields[0].parse().unwrap(),
                class_name: fields[1],
                method: fields[2],
                package_scheme: fields[3],
                package: fields[4],
                package_path: fields[5],
                line: fields[6].parse().unwrap(),
                column: fields[7].parse().unwrap(),
                is_constructor: fields[8].parse().unwrap(),
                source: fields[9],
            }
        })
        .collect()
}

fn assert_frame(actual: StackFrame<'_>, expected: StackFrame<'_>) {
    assert_eq!(actual, expected);
    // Flutter equality itself does not compare these three fields.
    assert_eq!(actual.package_scheme, expected.package_scheme);
    assert_eq!(actual.package_path, expected.package_path);
    assert_eq!(actual.is_constructor, expected.is_constructor);
}

#[test]
fn parses_numbered_vm_line_and_full_vm_stack() {
    let expected = fixture("stackFrames");
    assert_eq!(expected.len(), 14);
    let first = StackFrame::from_stack_trace_line(expected[0].source).unwrap();
    assert_frame(first, expected[0]);
    assert_eq!(
        first.to_string(),
        "StackFrame(#0, dart:core-patch/errors_patch.dart:42:39, className: _AssertionError, method: _doThrowNew)"
    );

    assert_stack(&expected);
}

#[test]
fn parses_full_async_vm_stack_without_losing_closures_or_markers() {
    let expected = fixture("asyncStackFrames");
    assert_eq!(expected.len(), 41);
    assert_stack(&expected);
}

#[test]
fn parses_full_stack_without_columns_and_with_absent_locations() {
    let expected = fixture("stackFrameNoColsFrames");
    assert_eq!(expected.len(), 10);
    assert_stack(&expected);
}

#[test]
fn parses_full_source_mapped_web_debug_stack() {
    let expected = fixture("webStackTraceFrames");
    assert_eq!(expected.len(), 14);
    assert_stack(&expected);
}

fn assert_stack(expected: &[StackFrame<'_>]) {
    let mut stack = String::from("\n\n");
    for frame in expected {
        stack.push_str(frame.source);
        stack.push('\n');
    }
    stack.push('\n');
    let actual = StackFrame::from_stack_string(&stack);
    assert_eq!(actual.len(), expected.len());
    for (actual, expected) in actual.into_iter().zip(expected.iter().copied()) {
        assert_frame(actual, expected);
    }
}

#[test]
fn canonical_gap_elision_and_invalid_markers() {
    assert_frame(
        StackFrame::from_stack_trace_line("...").unwrap(),
        StackFrame::STACK_OVERFLOW_ELISION,
    );
    assert_frame(
        StackFrame::from_stack_trace_line(ASYNC_GAP).unwrap(),
        StackFrame::ASYNCHRONOUS_SUSPENSION,
    );
    for invalid in [
        "===== asynchronous gap ==========================",
        "===== asynchronous gap =========================== ",
        "===== asynchronous frame ===========================",
        "wrong stack trace format",
        "#12 broken diagnostic",
    ] {
        assert_eq!(StackFrame::from_stack_trace_line(invalid), None);
    }
}

#[test]
fn package_stack_trace_chains_keep_one_canonical_gap() {
    // Rows from the pinned package:stack_trace test, including a Windows
    // path and anonymous callback spelling. Unknown surrounding lines skip.
    let trace = concat!(
        "Error: header, not a frame\n",
        include_str!("fixtures/flutter350_mangled_stack.txt")
    );
    let frames = StackFrame::from_stack_string(trace);
    assert_eq!(frames.len(), 12);
    assert_eq!(frames[1].method, "main.<fn>.<fn>");
    assert_eq!(frames[2].package, "flutter_test");
    assert_frame(frames[3], StackFrame::ASYNCHRONOUS_SUSPENSION);
}

#[test]
fn parses_http_constructor_without_a_class_and_hashes_flutter_identity_fields() {
    let source =
        "#32     new (http://localhost:42191/dart-sdk/lib/async/stream_controller.dart:880:9)";
    let expected = StackFrame {
        number: 32,
        class_name: "<unknown>",
        method: "",
        package_scheme: "http",
        package: "<unknown>",
        package_path: "/dart-sdk/lib/async/stream_controller.dart",
        line: 880,
        column: 9,
        is_constructor: true,
        source,
    };
    let actual = StackFrame::from_stack_trace_line(source).unwrap();
    assert_frame(actual, expected);
    let same_identity = StackFrame {
        package_scheme: "another",
        package_path: "elsewhere",
        is_constructor: false,
        ..expected
    };
    let hash = |frame: StackFrame<'_>| {
        let mut hasher = DefaultHasher::new();
        frame.hash(&mut hasher);
        hasher.finish()
    };
    assert_eq!(actual, same_identity);
    assert_eq!(hash(actual), hash(same_identity));
    assert_ne!(
        actual,
        StackFrame {
            column: 10,
            ..expected
        }
    );
}

#[test]
fn release_web_skips_exception_headers_and_keeps_compiled_names_without_coordinates() {
    let frames = StackFrame::parse_stack(
        "TypeError: cannot read property\n   at App.state.build$1 (http://localhost:8080/main.dart.js:12:30)\n   at k (http://localhost:8080/main.dart.js:20:1)",
        StackFrameFormat::WebNonDebug,
    );
    assert_eq!(frames.len(), 2);
    assert_eq!(
        (frames[0].class_name, frames[0].method),
        ("App", "state.build$1")
    );
    assert_eq!(
        (frames[0].number, frames[0].line, frames[0].column),
        (-1, -1, -1)
    );
    assert_eq!((frames[1].class_name, frames[1].method), ("<unknown>", "k"));
}
