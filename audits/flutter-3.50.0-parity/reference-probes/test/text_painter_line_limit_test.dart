import 'package:flutter/painting.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  for (final TextDirection direction in TextDirection.values) {
    final bool rtl = direction == TextDirection.rtl;
    void check({
      required String text,
      required int maxLines,
      required double maxWidth,
      required Size size,
      required double intrinsic,
      required bool exceeded,
      required bool lastHardBreak,
      required int firstHiddenOffset,
      required Offset hiddenCaret,
    }) {
      final painter = TextPainter(
        text: TextSpan(text: text, style: const TextStyle(fontSize: 10)),
        textDirection: direction,
        maxLines: maxLines,
      )..layout(maxWidth: maxWidth);
      expect(painter.plainText, text);
      expect(painter.size, size);
      expect(painter.maxIntrinsicWidth, intrinsic);
      expect(painter.didExceedMaxLines, exceeded);
      expect(painter.computeLineMetrics(), hasLength(maxLines));
      expect(painter.computeLineMetrics().last.hardBreak, lastHardBreak);
      const caret = Rect.fromLTWH(0, 0, 2, 0);
      for (int i = firstHiddenOffset; i <= text.length; i++) {
        for (final TextAffinity affinity in TextAffinity.values) {
          final position = TextPosition(offset: i, affinity: affinity);
          expect(painter.getOffsetForCaret(position, caret), hiddenCaret);
          expect(painter.getFullHeightForCaret(position, caret), 10);
        }
      }
      painter.maxLines = null;
      painter.layout(maxWidth: maxWidth);
      expect(painter.computeLineMetrics().length, greaterThan(maxLines));
      expect(painter.didExceedMaxLines, isFalse);
      painter.dispose();
    }

    test('hidden soft-wrapped source uses the $direction paragraph edge', () {
      check(
        text: 'abcd efgh ijkl',
        maxLines: 1,
        maxWidth: 40,
        size: const Size(40, 10),
        intrinsic: 140,
        exceeded: true,
        lastHardBreak: false,
        firstHiddenOffset: 6,
        hiddenCaret: Offset(rtl ? 0 : 40, 0),
      );
    });

    test(
      'text hidden after a hard line break uses the $direction last line',
      () {
        check(
          text: 'ab\ncd\nefgh',
          maxLines: 2,
          maxWidth: 100,
          size: const Size(40, 20),
          intrinsic: 40,
          exceeded: true,
          lastHardBreak: true,
          firstHiddenOffset: 6,
          hiddenCaret: Offset(rtl ? 18 : 20, 10),
        );
      },
    );

    test('omitting only the final empty $direction line is not overflow', () {
      check(
        text: 'ab\n',
        maxLines: 1,
        maxWidth: 100,
        size: const Size(20, 10),
        intrinsic: 20,
        exceeded: false,
        lastHardBreak: true,
        firstHiddenOffset: 3,
        hiddenCaret: Offset(rtl ? 0 : 20, 0),
      );
    });
  }
}
