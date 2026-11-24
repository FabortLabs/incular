import 'dart:ui';

import 'package:flutter/painting.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('Alignment.alongSize is measured from the top-left origin', () {
    const size = Size(100, 60);
    expect(
      [
        Alignment.topLeft.alongSize(size),
        Alignment.center.alongSize(size),
        Alignment.bottomRight.alongSize(size),
        const Alignment(1.5, -2).alongSize(size),
      ],
      const [Offset.zero, Offset(50, 30), Offset(100, 60), Offset(125, -30)],
    );
  });
}
