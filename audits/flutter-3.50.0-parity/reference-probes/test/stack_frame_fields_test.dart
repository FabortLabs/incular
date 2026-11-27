import 'package:flutter/foundation.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('Numbered frames preserve actual URI fields and Flutter identity', () {
    final StackFrame httpFrame = StackFrame.fromStackTraceLine(
      '#32     new (http://localhost:42191/dart-sdk/lib/async/stream_controller.dart:880:9)',
    )!;
    expect(httpFrame.packagePath, '/dart-sdk/lib/async/stream_controller.dart');
    expect(httpFrame.isConstructor, isTrue);
    expect(httpFrame.packageScheme, 'http');
    expect(
      httpFrame,
      StackFrame(
        number: httpFrame.number,
        column: httpFrame.column,
        line: httpFrame.line,
        packageScheme: 'ignored',
        package: httpFrame.package,
        packagePath: 'ignored',
        className: httpFrame.className,
        method: httpFrame.method,
        source: httpFrame.source,
      ),
    );

    final StackFrame fileFrame = StackFrame.fromStackTraceLine(
      '#3      main (file:///path/to/flutter/packages/flutter/test/foundation/error_reporting_test.dart:46:40)',
    )!;
    expect(
      fileFrame.packagePath,
      '/path/to/flutter/packages/flutter/test/foundation/error_reporting_test.dart',
    );
    expect(fileFrame.packageScheme, 'file');
    expect(fileFrame.isConstructor, isFalse);

    final StackFrame isolateFrame = StackFrame.fromStackTraceLine(
      '#8      _startIsolate.<anonymous closure> (dart:isolate-patch/isolate_patch.dart:307)',
    )!;
    expect(isolateFrame.packagePath, 'isolate_patch.dart');
    expect(isolateFrame.column, -1);
  });
}
