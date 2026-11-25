import 'package:flutter/painting.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('a beveled border strokes both the outer and the inner contour', () {
    final canvas = TestRecordingCanvas();
    const border = BeveledRectangleBorder(
      side: BorderSide(width: 4, strokeAlign: BorderSide.strokeAlignCenter),
      borderRadius: BorderRadius.all(Radius.circular(10)),
    );
    border.paint(canvas, const Rect.fromLTWH(0, 0, 100, 80));
    final invocation = canvas.invocations.single.invocation;
    expect(invocation.memberName, #drawPath);
    final path = invocation.positionalArguments[0] as Path;
    final paint = invocation.positionalArguments[1] as Paint;
    expect(path.computeMetrics().map((metric) => metric.isClosed), [
      true,
      true,
    ]);
    expect(path.getBounds(), const Rect.fromLTRB(-2, -2, 102, 82));
    expect(paint.style, PaintingStyle.stroke);
    expect(paint.strokeWidth, 4);
  });

  test('a zero-width rect does not emit a beveled border paint command', () {
    final canvas = TestRecordingCanvas();
    const border = BeveledRectangleBorder(side: BorderSide(width: 4));
    border.paint(canvas, const Rect.fromLTWH(0, 0, 0, 80));
    expect(canvas.invocations, isEmpty);
  });

  test('a beveled border can be wider than its nonempty rectangle', () {
    final canvas = TestRecordingCanvas();
    const border = BeveledRectangleBorder(side: BorderSide(width: 10));
    border.paint(canvas, const Rect.fromLTWH(0, 0, 2, 3));
    expect(canvas.invocations.single.invocation.memberName, #drawPath);
  });
}
