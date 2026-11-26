import 'package:flutter/painting.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('a superellipse morph queries a different inner edge from its painted stroke', () {
    const side = BorderSide(width: 4);
    const source = RoundedSuperellipseBorder(
      side: side,
      borderRadius: BorderRadius.all(Radius.circular(10)),
    );
    const target = CircleBorder(side: side);
    const area = Rect.fromLTWH(0, 0, 100, 20);
    final morph = ShapeBorder.lerp(source, target, 0.5)!;
    expect(morph.dimensions, const EdgeInsets.all(4));
    expect(
      morph.getOuterPath(area).getBounds(),
      const Rect.fromLTRB(20, 0, 80, 20),
    );
    expect(
      morph.getInnerPath(area).getBounds(),
      const Rect.fromLTRB(28, 8, 72, 12),
    );
    expect(
      (Canvas canvas) => morph.paint(canvas, area),
      paints..rsuperellipse(
        rsuperellipse: RSuperellipse.fromRectAndRadius(
          const Rect.fromLTRB(22, 2, 78, 18),
          const Radius.circular(8),
        ),
        strokeWidth: 4,
      ),
    );
  });
}
