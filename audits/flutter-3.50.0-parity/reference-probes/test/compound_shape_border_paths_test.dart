import 'package:flutter/painting.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('A compound border has the first outer and final inner path', () {
    const inside = RoundedRectangleBorder(side: BorderSide(width: 3));
    const outside = RoundedRectangleBorder(side: BorderSide(width: 2));
    final ShapeBorder compound = inside + outside;
    const rect = Rect.fromLTWH(0, 0, 40, 30);

    expect(
      compound.dimensions.resolve(TextDirection.ltr),
      const EdgeInsets.all(5),
    );
    expect(compound.getOuterPath(rect).getBounds(), rect);
    expect(
      compound.getInnerPath(rect).getBounds(),
      const Rect.fromLTWH(5, 5, 30, 20),
    );
  });
}
