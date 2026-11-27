import 'dart:ui' as ui;
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

const distances = <double>[0.35, 1.35, 3.35, 4.35, 7.35, 12.35, 20.35, 35.35, 50.35, 72.35, 100.35];

String mask(bool Function(Offset) contains, Rect rect, int corner) {
  final bool right = corner == 1 || corner == 2;
  final bool bottom = corner >= 2;
  final StringBuffer cells = StringBuffer();
  for (final double y in distances) {
    for (final double x in distances) {
      final Offset point = Offset(right ? rect.right - x : rect.left + x,
          bottom ? rect.bottom - y : rect.top + y);
      cells.write(contains(point) ? '1' : '0');
    }
  }
  return cells.toString();
}

void main() {
  test('Superellipse filled and inset paths at sharp, clamped and extreme corners', () {
    const Rect common = Rect.fromLTWH(7.25, 13.5, 120, 300);
    const Rect wide = Rect.fromLTWH(-99200, 50, 100000, 500);
    const fixtures = <(String, Rect, BorderRadius, double)>[
      ('zero', common, BorderRadius.zero, 4),
      ('circular', common, BorderRadius.all(Radius.circular(36)), 4),
      ('elliptical', common, BorderRadius.all(Radius.elliptical(20, 50)), 4),
      ('clamp-uniform', common, BorderRadius.all(Radius.circular(600)), 4),
      ('clamp-asymmetric', common, BorderRadius.only(
        topLeft: Radius.elliptical(1000, 1000),
        topRight: Radius.elliptical(0, 1000),
        bottomRight: Radius.elliptical(800, 1000),
        bottomLeft: Radius.elliptical(100, 500),
      ), 4),
      ('sharp-bottom', common, BorderRadius.only(
        topLeft: Radius.circular(16), topRight: Radius.circular(16),
      ), 4),
      ('very-wide', wide, BorderRadius.only(
        topRight: Radius.circular(100), bottomRight: Radius.circular(30),
      ), 1),
    ];
    for (final (String name, Rect rect, BorderRadius radius, double width) in fixtures) {
      final border = RoundedSuperellipseBorder(side: BorderSide(width: width), borderRadius: radius);
      final ui.Path outer = border.getOuterPath(rect);
      final ui.Path inner = border.getInnerPath(rect);
      expect(outer.contains(rect.center), isTrue, reason: name);
      expect(inner.contains(rect.center), isTrue, reason: name);
      for (int corner = 0; corner < 4; ++corner) {
        // These rows are the actual engine's Path.contains results. The native
        // test retains each cell, not just a hash or a geometry type name.
        print('SHAPE_MASK $name outer $corner ${mask(outer.contains, rect, corner)}');
        print('SHAPE_MASK $name inner $corner ${mask(inner.contains, rect, corner)}');
        print('SHAPE_MASK $name hit $corner ${mask((Offset point) => border.hitTest(rect, point, textDirection: TextDirection.ltr), rect, corner)}');
      }
    }
  });


  testWidgets('A painted clamped corner is not necessarily a decoration hit', (WidgetTester tester) async {
    var taps = 0;
    const Key box = ValueKey<String>('clamped shape');
    const Size extent = Size(120, 300);
    const RoundedSuperellipseBorder border = RoundedSuperellipseBorder(
      borderRadius: BorderRadius.all(Radius.circular(600)),
    );
    await tester.pumpWidget(Center(
      child: GestureDetector(
        behavior: HitTestBehavior.deferToChild,
        onTap: () => taps++,
        child: const DecoratedBox(
          key: box,
          decoration: ShapeDecoration(shape: border, color: Color(0xff787878)),
          child: SizedBox(width: 120, height: 300),
        ),
      ),
    ));
    const Offset localPaintOnly = Offset(50.35, 1.35);
    final Rect rect = Offset.zero & extent;
    expect(border.getOuterPath(rect).contains(localPaintOnly), isTrue);
    expect(border.hitTest(rect, localPaintOnly, textDirection: TextDirection.ltr), isFalse);
    final Offset origin = tester.getTopLeft(find.byKey(box));
    await tester.tapAt(origin + localPaintOnly);
    expect(taps, 0);
    await tester.tapAt(origin + rect.center);
    expect(taps, 1);
  });
}
