import 'package:flutter/painting.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('The native superellipse inset has the expected edge geometry', () {
    const shape = RoundedSuperellipseBorder(
      side: BorderSide(width: 2),
      borderRadius: BorderRadius.all(Radius.circular(20)),
    );
    const Rect area = Rect.fromLTWH(0, 0, 100, 100);
    final Path outer = shape.getOuterPath(area);
    final Path inner = shape.getInnerPath(area);
    expect(outer.getBounds(), area);
    expect(inner.getBounds(), const Rect.fromLTWH(2, 2, 96, 96));
    expect(outer.contains(const Offset(5.9, 5.9)), isTrue);
    expect(outer.contains(const Offset(5.8, 5.8)), isFalse);
    final samples = <double, List<bool>>{
      20: [false, false, false, false, false, false],
      25: [false, false, false, false, false, false],
      30: [false, false, false, false, false, true],
      35: [false, false, false, false, true, true],
      40: [false, false, false, false, true, true],
      45: [false, false, false, false, true, true],
      50: [false, false, false, true, true, true],
    };
    const heights = <double>[1.9, 1.95, 1.99, 2.0, 2.01, 2.05];
    for (final MapEntry<double, List<bool>> sample in samples.entries) {
      expect(
        heights.map((y) => inner.contains(Offset(sample.key, y))).toList(),
        sample.value,
        reason: 'edge at x=${sample.key}',
      );
    }
  });
}
