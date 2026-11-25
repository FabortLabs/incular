import 'package:flutter/painting.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('a thick continuous border can have inverted inner bounds', () {
    const border = ContinuousRectangleBorder(
      side: BorderSide(width: 10),
      borderRadius: BorderRadius.all(Radius.circular(5)),
    );
    const rect = Rect.fromLTWH(0, 0, 2, 3);
    expect(
      border.getInnerPath(rect).getBounds(),
      const Rect.fromLTRB(-8, -7, 10, 10),
    );
    final canvas = TestRecordingCanvas();
    border.paint(canvas, rect);
    final call = canvas.invocations.single.invocation;
    expect(call.memberName, #drawPath);
    expect((call.positionalArguments.first as Path).getBounds(), rect);
    expect((call.positionalArguments[1] as Paint).strokeWidth, 10);
  });

  test(
    'continuous elliptical top-left uses x vertically and y horizontally',
    () {
      const border = ContinuousRectangleBorder(
        borderRadius: BorderRadius.only(topLeft: Radius.elliptical(10, 3)),
      );
      expect(
        border.getOuterPath(const Rect.fromLTWH(0, 0, 100, 80)),
        isPathThat(
          includes: const [Offset(3, 0), Offset(0, 10), Offset(4, 0.25)],
          excludes: const [Offset(0, 5), Offset(2, 0)],
        ),
      );
    },
  );
}
