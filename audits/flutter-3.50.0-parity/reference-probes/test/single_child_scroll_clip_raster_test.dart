import 'dart:ui' as ui;

import 'package:flutter/rendering.dart' show RenderRepaintBoundary;
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('scroll viewport clips layout overflow but not paint-only shift', (tester) async {
    final key = GlobalKey();
    const red = SizedBox(width: 20, height: 20,
      child: ColoredBox(color: Color(0xffff0000)));
    for (final (layout, clip) in [
      (false, Clip.hardEdge), (true, Clip.hardEdge), (false, Clip.hardEdge),
      (true, Clip.none), (true, Clip.hardEdge), (false, Clip.none),
    ]) {
      final child = layout ? const SizedBox(
        width: 40, height: 50,
        child: Stack(children: [Positioned(left: 0, top: 30, width: 20, height: 20, child: red)]),
      ) : Transform.translate(offset: const Offset(0, 30), child: red);
      await tester.pumpWidget(Directionality(
        textDirection: TextDirection.ltr,
        child: Center(child: RepaintBoundary(key: key,
          child: SizedBox(width: 100, height: 100, child: ColoredBox(
            color: const Color(0xff000000),
            child: Center(child: SizedBox(width: 40, height: 40,
              child: SingleChildScrollView(clipBehavior: clip, child: child),
            )),
          )),
        )),
      ));
      final boundary = tester.renderObject<RenderRepaintBoundary>(find.byKey(key));
      final data = (await tester.runAsync(() async {
        final image = await boundary.toImage(pixelRatio: 1);
        try {
          expect((image.width, image.height), (100, 100));
          return (await image.toByteData(format: ui.ImageByteFormat.rawRgba))!;
        } finally {
          image.dispose();
        }
      }))!;
      for (final (y, r) in [
        (59,0),(65,255),(75,layout && clip == Clip.hardEdge ? 0 : 255),(80,0),
      ]) {
        final offset = (y * 100 + 40) * 4;
        expect(List.generate(4, (i) => data.getUint8(offset + i)), [r,0,0,255],
          reason: 'layout=$layout, clip=$clip, y=$y');
      }
    }
  });
}
