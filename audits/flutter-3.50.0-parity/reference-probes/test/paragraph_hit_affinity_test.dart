import 'package:flutter/painting.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('paragraph affinity at a visual glyph boundary', () {
    for (final (direction, text, left, right) in [
      (
        TextDirection.ltr,
        '1234567',
        TextAffinity.upstream,
        TextAffinity.downstream,
      ),
      (
        TextDirection.rtl,
        'אבגדהוז',
        TextAffinity.downstream,
        TextAffinity.upstream,
      ),
    ]) {
      final painter = TextPainter(
        text: TextSpan(text: text, style: const TextStyle(fontSize: 14)),
        textDirection: direction,
      )..layout();
      final box = painter
          .getBoxesForSelection(
            const TextSelection(baseOffset: 3, extentOffset: 4),
          )
          .single;
      final x = direction == TextDirection.ltr ? box.left : box.right;
      for (final (dx, affinity) in [(-1.0, left), (0.0, right), (1.0, right)]) {
        expect(
          painter.getPositionForOffset(Offset(x + dx, 14)),
          TextPosition(offset: 3, affinity: affinity),
          reason: '$direction at ${x + dx}',
        );
      }
      painter.dispose();
    }
  });
}
