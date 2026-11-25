import 'package:flutter/painting.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('An incompatible left border keeps the right operand outside', () {
    const magenta = Color(0xffff00ff);
    const yellow = Color(0xffffff00);
    const inside = Border(left: BorderSide(color: magenta, width: 3));
    const outside = Border(left: BorderSide(color: yellow, width: 2));
    const area = Rect.fromLTWH(0, 0, 30, 20);
    final ShapeBorder compound = inside + outside;
    expect(compound, isNot(isA<Border>()));
    expect(
      compound.getInnerPath(area, textDirection: TextDirection.ltr).getBounds(),
      const Rect.fromLTWH(5, 0, 25, 20),
    );
    expect(
      (Canvas canvas) =>
          compound.paint(canvas, area, textDirection: TextDirection.ltr),
      paints
        ..path(
          color: yellow,
          includes: const [Offset(1, 10)],
          excludes: const [Offset(3, 10)],
        )
        ..path(
          color: magenta,
          includes: const [Offset(3, 10)],
          excludes: const [Offset(1, 10)],
        ),
    );
  });
}
