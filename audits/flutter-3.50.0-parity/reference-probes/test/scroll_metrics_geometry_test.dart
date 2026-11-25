import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('ScrollMetrics distinguishes exact edges from overscroll', () {
    FixedScrollMetrics metrics(double pixels, {double max = 200}) =>
        FixedScrollMetrics(
          minScrollExtent: 0,
          maxScrollExtent: max,
          pixels: pixels,
          viewportDimension: 100,
          axisDirection: AxisDirection.down,
          devicePixelRatio: 2,
        );

    expect(metrics(-20).extentInside, 80);
    final actual = [-120.0, -20.0, 0.0, 35.0, 200.0, 220.0, 320.0].map((
      pixels,
    ) {
      final value = metrics(pixels);
      return (value.atEdge, value.outOfRange, value.extentInside);
    }).toList();
    expect(actual, [
      (false, true, 0.0),
      (false, true, 80.0),
      (true, false, 100.0),
      (false, false, 100.0),
      (true, false, 100.0),
      (false, true, 80.0),
      (false, true, 0.0),
    ]);
    expect(metrics(0, max: 0).extentInside, 100);
    expect(metrics(0, max: 0).atEdge, isTrue);
    expect(metrics(20, max: 0).extentInside, 80);
    expect(metrics(20, max: 0).atEdge, isFalse);
  });
}
