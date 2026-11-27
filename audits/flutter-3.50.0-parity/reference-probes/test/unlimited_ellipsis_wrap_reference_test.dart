import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  TextPainter ellipsis(String text, {double width = 45, int? maxLines}) => TextPainter(
    text: TextSpan(text: text, style: const TextStyle(fontSize: 10, height: 1)),
    textDirection: TextDirection.ltr,
    ellipsis: '…',
    maxLines: maxLines,
  )..layout(maxWidth: width);

  test('Unlimited ellipsis retains all hard lines when they fit', () {
    final painter = ellipsis('one\ntwo\nthin');
    addTearDown(painter.dispose);
    expect(painter.computeLineMetrics().length, 3);
    expect(painter.didExceedMaxLines, isFalse);
  });

  test('Unlimited ellipsis stops at the first hard line exceeding the width', () {
    final painter = ellipsis('one\nthin enormousword after');
    addTearDown(painter.dispose);
    expect(painter.computeLineMetrics().length, 2);
    expect(painter.didExceedMaxLines, isTrue);
    expect(painter.getBoxesForSelection(
      const TextSelection(baseOffset: 4, extentOffset: 7),
    ), isNotEmpty);
    expect(painter.getBoxesForSelection(
      const TextSelection(baseOffset: 7, extentOffset: 26),
    ), isEmpty);
  });

  test('Unlimited ellipsis does not soft-wrap onto another line', () {
    final painter = ellipsis('one two\nthin');
    addTearDown(painter.dispose);
    expect(painter.computeLineMetrics().length, 1);
    expect(painter.didExceedMaxLines, isTrue);
    expect(painter.getBoxesForSelection(
      const TextSelection(baseOffset: 4, extentOffset: 12),
    ), isEmpty);
  });

  test('Ellipsis itself is omitted when it cannot fit', () {
    for (final cap in <int?>[null, 1]) {
      final painter = ellipsis('text', width: 3, maxLines: cap);
      addTearDown(painter.dispose);
      expect(painter.didExceedMaxLines, isTrue);
      expect(painter.computeLineMetrics().single.width, 0);
      expect(painter.getBoxesForSelection(
        const TextSelection(baseOffset: 0, extentOffset: 4),
      ), isEmpty);
    }
  });

  test('A long unbroken line retains the same first three glyphs', () {
    final painter = ellipsis('W' * 4096);
    addTearDown(painter.dispose);
    expect(painter.computeLineMetrics().length, 1);
    expect(painter.didExceedMaxLines, isTrue);
    expect(painter.getBoxesForSelection(
      const TextSelection(baseOffset: 0, extentOffset: 3),
    ), isNotEmpty);
    expect(painter.getBoxesForSelection(
      const TextSelection(baseOffset: 3, extentOffset: 4096),
    ), isEmpty);
  });
}
