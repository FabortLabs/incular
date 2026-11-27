import 'package:flutter/foundation.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('Default filtering preserves user frames and counts dropped internal frames', () {
    const List<String> trace = <String>[
      'unrecognized header',
      '#0      Widget.build (package:app/ui.dart:2:5)',
      '#1      _AssertionError._raise (dart:async/future.dart:5:1)',
      '#2      _rootRun (dart:async/zone.dart:7:2)',
      '===== asynchronous gap ===========================',
      '#3      Chain.capture (package:stack_trace/src/chain.dart:9:1)',
      '#4      main (file:///host/app.dart:10:2)',
    ];
    expect(FlutterError.defaultStackFilter(trace), <String>[
      trace[1],
      '<asynchronous suspension>',
      trace.last,
      '(elided 3 frames from class _AssertionError, dart:async, and package:stack_trace)',
    ]);
    expect(FlutterError.defaultStackFilter(<String>[trace[2]]), <String>[
      '(elided one frame from class _AssertionError)',
    ]);
  });

  test('Printing labels first and applies maxFrames before filtering', () {
    final List<String?> messages = <String?>[];
    final DebugPrintCallback previous = debugPrint;
    debugPrint = (String? message, {int? wrapWidth}) => messages.add(message);
    try {
      debugPrintStack(
        label: 'Prefix',
        maxFrames: 3,
        stackTrace: StackTrace.fromString(
          '#0      first (package:app/app.dart:1:2)\n'
          '#1      _rootRun (dart:async/zone.dart:3:4)\n'
          '===== asynchronous gap ===========================\n'
          '#3      later (package:app/app.dart:5:6)\n',
        ),
      );
    } finally {
      debugPrint = previous;
    }
    expect(messages, <String>[
      'Prefix',
      '#0      first (package:app/app.dart:1:2)\n'
      '<asynchronous suspension>\n'
      '(elided one frame from dart:async)',
    ]);
  });

  test('Overlapping stack filters follow the pinned scan and summary behavior', () {
    const PartialStackFrame first = PartialStackFrame(
      package: 'package:app/ui.dart', className: 'Widget', method: 'build',
    );
    const PartialStackFrame second = PartialStackFrame(
      package: 'package:app/ui.dart', className: 'Widget', method: 'paint',
    );
    const trace = <String>[
      '#0      Widget.build (package:app/ui.dart:1:1)',
      '#1      Widget.paint (package:app/ui.dart:2:1)',
      '#2      irrelevant (package:app/ui.dart:3:1)',
      '#3      Widget.build (package:app/ui.dart:4:1)',
      '#4      Widget.paint (package:app/ui.dart:5:1)',
    ];
    const RepetitiveStackFrameFilter pattern = RepetitiveStackFrameFilter(
      frames: <PartialStackFrame>[first, second], replacement: 'matched',
    );
    final List<String?> reasons = List<String?>.filled(trace.length, null);
    pattern.filter(StackFrame.fromStackString(trace.join('\n')), reasons);
    expect(reasons, <String?>['matched', 'matched', null, null, null]);

    FlutterError.addDefaultStackFilter(const RepetitiveStackFrameFilter(
      frames: <PartialStackFrame>[first, second], replacement: 'earlier',
    ));
    FlutterError.addDefaultStackFilter(pattern);
    expect(FlutterError.defaultStackFilter(trace), <String>[
      'matched (3 frames)', trace[2], trace[3], trace[4],
    ]);
  });
}
