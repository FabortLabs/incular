use incular_core::{
    DiagnosticStack, RepetitiveStackFrameFilter, StackFilter, StackFrame, StackFrameOrigin,
    StackFramePattern, StackPackagePattern,
};

const MANGLED_STACK: &str = include_str!("fixtures/flutter350_mangled_stack.txt");

#[test]
fn default_filter_drops_internal_frames_and_sorts_summary_reasons() {
    let diagnostics = DiagnosticStack::default();
    let trace = concat!(
        "unrecognized header\n",
        "#0      Widget.build (package:app/ui.dart:2:5)\n",
        "#1      _AssertionError._raise (dart:async/future.dart:5:1)\n",
        "#2      _rootRun (dart:async/zone.dart:7:2)\n",
        "===== asynchronous gap ===========================\n",
        "#3      Chain.capture (package:stack_trace/src/chain.dart:9:1)\n",
        "#4      main (file:///host/app.dart:10:2)\n",
    );
    assert_eq!(
        diagnostics.default_stack_filter(trace),
        [
            "#0      Widget.build (package:app/ui.dart:2:5)",
            "<asynchronous suspension>",
            "#4      main (file:///host/app.dart:10:2)",
            "(elided 3 frames from class _AssertionError, dart:async, and package:stack_trace)",
        ]
    );
    assert_eq!(
        diagnostics
            .default_stack_filter("#1      _AssertionError._raise (dart:async/future.dart:5:1)"),
        ["(elided one frame from class _AssertionError)"]
    );
    assert_eq!(
        diagnostics.default_stack_filter(
            "#0      _Timer.run (dart:async/timer.dart:1)\n#1      _rootRun (dart:async/zone.dart:2)"
        ),
        ["(elided 2 frames from class _Timer and dart:async)"]
    );
    assert_eq!(
        diagnostics.default_stack_filter(
            "#0      _rootRun (dart:async/zone.dart:1)\n#1      _rootRun (dart:async/zone.dart:2)"
        ),
        ["(elided 2 frames from dart:async)"]
    );
}

#[test]
fn prints_original_mangled_stack_with_a_label_and_an_async_gap() {
    let mut messages = Vec::new();
    DiagnosticStack::default().debug_print_stack(
        MANGLED_STACK,
        Some("ParallelWaitError-style stack"),
        None,
        |message| messages.push(message.to_owned()),
    );
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0], "ParallelWaitError-style stack");
    assert_eq!(messages[1].lines().count(), 12);
    assert_eq!(
        messages[1].lines().nth(3),
        Some("<asynchronous suspension>")
    );
}

#[test]
fn frame_limit_is_applied_before_filters_and_web_capture_prefix_is_skipped() {
    let mut messages = Vec::new();
    DiagnosticStack::default().debug_print_stack(
        concat!(
            "#0      first (package:app/app.dart:1:2)\n",
            "#1      _rootRun (dart:async/zone.dart:3:4)\n",
            "===== asynchronous gap ===========================\n",
            "#3      later (package:app/app.dart:5:6)\n",
        ),
        Some("Prefix"),
        Some(3),
        |message| messages.push(message.to_owned()),
    );
    assert_eq!(
        messages,
        [
            "Prefix",
            "#0      first (package:app/app.dart:1:2)\n<asynchronous suspension>\n(elided one frame from dart:async)"
        ]
    );
    messages.clear();
    let web = DiagnosticStack {
        origin: StackFrameOrigin::WebDebug,
        ..DiagnosticStack::default()
    };
    web.debug_print_stack(
        concat!(
            "dart-sdk/lib/_internal/js_runtime.dart 1:2  StackTrace.current\n",
            "dart:sdk_internal 3:4  hook\n",
            "package:app/app.dart 5:6  main\n",
            "package:app/app.dart 7:8  later\n"
        ),
        None,
        Some(1),
        |message| messages.push(message.to_owned()),
    );
    assert_eq!(messages, ["package:app/app.dart 5:6  main"]);
    messages.clear();
    web.debug_print_stack("...", None, Some(0), |message| {
        messages.push(message.to_owned())
    });
    assert_eq!(messages, [""]);
}

#[test]
fn repeated_pattern_longer_than_trace_never_writes_out_of_range() {
    let pattern = ["test1", "test2", "test3"].map(|method| StackFramePattern {
        package: StackPackagePattern::Contains("package:test/blah.dart"),
        class_name: "TestClass",
        method,
    });
    let filter = RepetitiveStackFrameFilter {
        frames: &pattern,
        replacement: "test",
    };
    let frames = StackFrame::from_stack_string(concat!(
        "#0      TestClass.test1 (package:test/blah.dart:1:1)\n",
        "#0      TestClass.test2 (package:test/blah.dart:1:1)"
    ));
    let mut reasons = [None; 2];
    filter.filter(&frames, &mut reasons, StackFrameOrigin::Native);
    assert_eq!(reasons, [None, None]);
}

#[test]
fn overlapping_filters_preserve_pinned_terminal_window_and_summary_behavior() {
    let pattern = ["build", "paint"].map(|method| StackFramePattern {
        package: StackPackagePattern::Contains("package:app/ui.dart"),
        class_name: "Widget",
        method,
    });
    let earlier = RepetitiveStackFrameFilter {
        frames: &pattern,
        replacement: "earlier",
    };
    let matched = RepetitiveStackFrameFilter {
        frames: &pattern,
        replacement: "matched",
    };
    let trace = concat!(
        "#0      Widget.build (package:app/ui.dart:1:1)\n",
        "#1      Widget.paint (package:app/ui.dart:2:1)\n",
        "#2      irrelevant (package:app/ui.dart:3:1)\n",
        "#3      Widget.build (package:app/ui.dart:4:1)\n",
        "#4      Widget.paint (package:app/ui.dart:5:1)"
    );
    let diagnostics = DiagnosticStack {
        origin: StackFrameOrigin::Native,
        filters: &[&earlier, &matched],
    };
    assert_eq!(
        diagnostics.default_stack_filter(trace),
        [
            "matched (3 frames)",
            "#2      irrelevant (package:app/ui.dart:3:1)",
            "#3      Widget.build (package:app/ui.dart:4:1)",
            "#4      Widget.paint (package:app/ui.dart:5:1)",
        ]
    );
}

#[test]
fn web_private_methods_use_brackets_without_a_known_class() {
    let pattern = StackFramePattern {
        package: StackPackagePattern::Predicate(&|package| package.ends_with("/app.dart")),
        class_name: "Application",
        method: "_paint",
    };
    let bracketed =
        StackFrame::from_stack_trace_line("package:app/app.dart 5:6  [_paint]").unwrap();
    assert!(pattern.matches(&bracketed, StackFrameOrigin::WebDebug));
    assert!(!pattern.matches(&bracketed, StackFrameOrigin::Native));
    assert!(StackFramePattern::ASYNCHRONOUS_SUSPENSION.matches(
        &StackFrame::ASYNCHRONOUS_SUSPENSION,
        StackFrameOrigin::Native
    ));
}

#[test]
fn stack_trace_as_a_diagnostic_block_respects_the_heading_separator() {
    let stack = concat!(
        "#0      someMethod  (file:///diagnostics_test.dart:42:19)\n",
        "#1      someMethod2  (file:///diagnostics_test.dart:12:3)\n",
        "#2      someMethod3  (file:///foo.dart:4:1)\n",
    );
    let printer = DiagnosticStack::default();
    assert_eq!(
        printer.format_diagnostic_block("Stack trace", Some(stack), true),
        format!("Stack trace:\n{stack}"),
    );
    assert_eq!(
        printer.format_diagnostic_block("-- callback 2 --", Some(stack), false),
        format!("-- callback 2 --\n{stack}"),
    );
}
