import 'dart:ui' as ui;

import 'package:flutter/rendering.dart' show RenderRepaintBoundary;
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('none border keeps two-pixel insets without drawing', (tester) async {
    final key = GlobalKey();
    const tint = Color(0xffd90b4f);
    const white = Color(0xffffffff);
    const black = Color(0xff000000);
    for (final (style, edge) in [
      (null, white), (BorderStyle.solid, tint), (BorderStyle.none, black), (null, white),
    ]) {
      await tester.pumpWidget(Directionality(
        textDirection: TextDirection.ltr,
        child: Center(child: RepaintBoundary(
          key: key,
          child: SizedBox(width: 60, height: 60, child: ColoredBox(
            color: black,
            child: Center(child: Container(
              width: 40,
              height: 40,
              decoration: BoxDecoration(border: style == null ? null :
                Border.fromBorderSide(BorderSide(color: tint, width: 2, style: style))),
              child: const ColoredBox(color: white),
            )),
          )),
        )),
      ));
      final boundary = tester.renderObject<RenderRepaintBoundary>(find.byKey(key));
      final data = (await tester.runAsync(() async {
        final image = await boundary.toImage(pixelRatio: 1);
        try {
          expect((image.width,image.height), (60,60));
          return (await image.toByteData(format: ui.ImageByteFormat.rawRgba))!;
        } finally {
          image.dispose();
        }
      }))!;
      for (final (x, expected) in [
        (9,black),(10,edge),(11,edge),(12,white),(47,white),(48,edge),(49,edge),(50,black),
      ]) {
        final base = (30 * 60 + x) * 4;
        expect(List.generate(4, (i) => data.getUint8(base+i)), [
          (expected.toARGB32() >> 16) & 255, (expected.toARGB32() >> 8) & 255, expected.toARGB32() & 255, 255,
        ], reason: 'style=$style, x=$x');
      }
    }
  });
}
