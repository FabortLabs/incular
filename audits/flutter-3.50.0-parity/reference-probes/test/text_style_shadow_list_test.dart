import 'dart:ui' as ui;

import 'package:flutter/painting.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('unpaired shadows scale geometry but not color', () {
    const color = Color(0x800a141e);
    const a = TextStyle(
      shadows: [ui.Shadow(color: color, offset: Offset(-8, 12), blurRadius: 4)],
    );
    const b = TextStyle();
    expect(TextStyle.lerp(a, b, 0.25)?.shadows, [
      const ui.Shadow(color: color, offset: Offset(-6, 9), blurRadius: 3),
    ]);
    expect(TextStyle.lerp(b, a, 0.25)?.shadows, [
      const ui.Shadow(color: color, offset: Offset(-2, 3), blurRadius: 1),
    ]);
    expect(TextStyle.lerp(a, b, 1)?.shadows, [const ui.Shadow(color: color)]);
  });
}
