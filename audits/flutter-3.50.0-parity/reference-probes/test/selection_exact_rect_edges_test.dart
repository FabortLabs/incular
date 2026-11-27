import 'package:flutter/rendering.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test(
    'selection classification and snapping use distinct rules at exact edges',
    () {
      const rect = Rect.fromLTWH(100, 100, 200, 500);
      for (final (point, classification, adjusted) in [
        (const Offset(100, 100), SelectionResult.end, const Offset(100, 100)),
        (const Offset(300, 100), SelectionResult.next, const Offset(100, 100)),
        (const Offset(300, 300), SelectionResult.next, const Offset(300, 600)),
        (
          const Offset(100, 600),
          SelectionResult.previous,
          const Offset(100, 100),
        ),
        (
          const Offset(200, 600),
          SelectionResult.previous,
          const Offset(300, 600),
        ),
      ]) {
        expect(
          SelectionUtils.getResultBasedOnRect(rect, point),
          classification,
          reason: '$point',
        );
        expect(
          SelectionUtils.adjustDragOffset(rect, point),
          adjusted,
          reason: '$point',
        );
      }
    },
  );
}
