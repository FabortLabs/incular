import 'dart:ui' as ui;

import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('mounted paragraph paints a discretionary mark only in manual mode', (tester) async {
    final boundaryKey = GlobalKey();
    for (final (hyphens, showMark) in [
      (ui.Hyphens.manual, true),
      (ui.Hyphens.hidden, false),
      (ui.Hyphens.manual, true),
    ]) {
      await tester.pumpWidget(
        Directionality(
          textDirection: TextDirection.ltr,
          child: Align(
            alignment: Alignment.topLeft,
            child: RepaintBoundary(
              key: boundaryKey,
              child: SizedBox(
                width: 49,
                child: RichText(
                  text: const TextSpan(
                    text: 'abc\u00ADde',
                    style: TextStyle(fontSize: 10, color: Color(0xFF000000)),
                  ),
                  hyphens: hyphens,
                ),
              ),
            ),
          ),
        ),
      );
      final boundary = boundaryKey.currentContext!.findRenderObject()! as RenderRepaintBoundary;
      expect(boundary.size, const Size(49, 20));
      final bool? painted = await tester.runAsync(() async {
        final image = await boundary.toImage();
        try {
          final pixels = (await image.toByteData())!;
          return [
            for (int y = 0; y < 10; y += 1)
              for (int x = 30; x < 40; x += 1)
                pixels.getUint8(4 * (y * image.width + x) + 3),
          ].any((alpha) => alpha != 0);
        } finally {
          image.dispose();
        }
      });
      expect(painted, showMark, reason: '$hyphens');
    }
  });
}
