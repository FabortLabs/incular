import 'package:flutter/painting.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('narrow paragraph falls back to breaking an unbreakable word', () {
    final painter = TextPainter(
      text: TextSpan(text: 'a' * 100, style: const TextStyle(fontSize: 14)),
      textDirection: TextDirection.ltr,
    );
    addTearDown(painter.dispose);
    painter.layout(maxWidth: 30);
    final narrow = painter.computeLineMetrics();
    expect(narrow.length, greaterThanOrEqualTo(2));
    expect(narrow.every((line) => line.width <= 30), isTrue);
    painter.layout();
    final natural = painter.computeLineMetrics();
    expect(natural.length, 1);
    expect(natural.single.width, greaterThan(30));
  });
}
