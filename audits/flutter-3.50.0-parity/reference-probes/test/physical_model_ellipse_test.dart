import 'dart:ui' as ui;

import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('a non-square physical circle paints a full oval', (
    tester,
  ) async {
    await tester.pumpWidget(
      Directionality(
        textDirection: TextDirection.ltr,
        child: Align(
          alignment: Alignment.topLeft,
          child: SizedBox(
            width: 100,
            height: 40,
            child: PhysicalModel(
              color: const Color(0xff123456),
              shape: BoxShape.circle,
              child: const SizedBox.expand(),
            ),
          ),
        ),
      ),
    );
    final model = tester.allRenderObjects
        .whereType<RenderPhysicalModel>()
        .single;
    final expected = RRect.fromRectXY(
      const Rect.fromLTWH(0, 0, 100, 40),
      50,
      20,
    );
    expect((ui.Canvas canvas) {
      model.paint(TestRecordingPaintingContext(canvas), Offset.zero);
    }, paints..rrect(rrect: expected, color: const Color(0xff123456)));
    expect(expected.contains(const Offset(2, 20)), isTrue);
    expect(expected.contains(const Offset(98, 20)), isTrue);
    expect(expected.contains(const Offset(1, 1)), isFalse);
  });

  testWidgets('physical oval surface does not itself hit test', (tester) async {
    var taps = 0;
    await tester.pumpWidget(
      Directionality(
        textDirection: TextDirection.ltr,
        child: Center(
          child: GestureDetector(
            behavior: HitTestBehavior.deferToChild,
            onTap: () => taps++,
            child: const SizedBox(
              width: 100,
              height: 40,
              child: PhysicalModel(
                color: Color(0xff123456),
                shape: BoxShape.circle,
                child: SizedBox.expand(),
              ),
            ),
          ),
        ),
      ),
    );
    await tester.tapAt(const Offset(400, 300));
    expect(taps, 0);
  });
}
