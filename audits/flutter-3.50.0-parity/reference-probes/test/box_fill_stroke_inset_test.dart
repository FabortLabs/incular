// Authored for Incular parity; not an upstream test registration.
import 'dart:ui';

import 'package:flutter/painting.dart';
import 'package:flutter_test/flutter_test.dart';

class BoxCanvas implements Canvas {
  RRect? fill;
  (RRect, RRect)? ring;

  @override
  void drawRRect(RRect rrect, Paint paint) => fill = rrect;

  @override
  void drawDRRect(RRect outer, RRect inner, Paint paint) =>
      ring = (outer, inner);

  @override
  dynamic noSuchMethod(Invocation invocation) => super.noSuchMethod(invocation);
}

void main() {
  test('uniform box border insets opaque fills on the original box', () {
    for (final alpha in [255, 254]) {
      for (final gradient in [false, true]) {
        final canvas = BoxCanvas();
        final painter = BoxDecoration(
          color: gradient ? null : const Color(0xFF000000),
          gradient: gradient
              ? const LinearGradient(
                  colors: [Color(0xFF000000), Color(0xFFFF0000)],
                )
              : null,
          borderRadius: const BorderRadius.all(Radius.circular(75)),
          border: Border.all(
            color: Color.fromARGB(alpha, 255, 255, 255),
            width: 50,
          ),
        ).createBoxPainter(() {});
        painter.paint(
          canvas,
          Offset.zero,
          const ImageConfiguration(size: Size(250, 250)),
        );
        final inset = alpha == 255 ? 25.0 : 0.0;
        expect(
          canvas.fill!.outerRect,
          Rect.fromLTRB(inset, inset, 250 - inset, 250 - inset),
        );
        expect(canvas.fill!.tlRadius, const Radius.circular(75));
        expect(canvas.ring!.$1.outerRect, const Rect.fromLTWH(0, 0, 250, 250));
        expect(
          canvas.ring!.$2.outerRect,
          const Rect.fromLTWH(50, 50, 150, 150),
        );
        painter.dispose();
      }
    }
  });
}
