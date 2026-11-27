// Authored for Incular parity; not an upstream test registration.
import 'dart:ui';

import 'package:flutter/painting.dart';
import 'package:flutter_test/flutter_test.dart';

class CircleCanvas implements Canvas {
  final circles = <(Offset, double, PaintingStyle)>[];

  @override
  void drawCircle(Offset center, double radius, Paint paint) {
    circles.add((center, radius, paint.style));
  }

  @override
  dynamic noSuchMethod(Invocation invocation) => super.noSuchMethod(invocation);
}

void main() {
  test(
    'opaque solid outline insets circle color, not its gradient or stroke',
    () {
      for (final (alpha, align, style, gradientOnly, radius) in [
        (255, -1.0, BorderStyle.solid, false, 45.0),
        (255, 0.0, BorderStyle.solid, false, 47.5),
        (255, 1.0, BorderStyle.solid, false, 50.0),
        (254, -1.0, BorderStyle.solid, false, 50.0),
        (255, -1.0, BorderStyle.none, false, 50.0),
        (255, -1.0, BorderStyle.solid, true, 50.0),
      ]) {
        final canvas = CircleCanvas();
        final painter = ShapeDecoration(
          color: gradientOnly ? null : const Color(0xFFFF0000),
          gradient: gradientOnly
              ? const LinearGradient(
                  colors: [Color(0xFFFF0000), Color(0xFF0000FF)],
                )
              : null,
          shape: CircleBorder(
            side: BorderSide(
              color: Color.fromARGB(alpha, 255, 255, 255),
              width: 10,
              style: style,
              strokeAlign: align,
            ),
          ),
        ).createBoxPainter(() {});
        painter.paint(
          canvas,
          Offset.zero,
          const ImageConfiguration(size: Size(100, 100)),
        );
        expect(canvas.circles.first, (
          const Offset(50, 50),
          radius,
          PaintingStyle.fill,
        ));
        if (style == BorderStyle.solid) {
          expect(canvas.circles.last, (
            const Offset(50, 50),
            50 + align * 5,
            PaintingStyle.stroke,
          ));
        }
        painter.dispose();
      }
    },
  );
}
