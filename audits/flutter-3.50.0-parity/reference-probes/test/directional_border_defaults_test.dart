import 'package:flutter/painting.dart';
import 'package:flutter_test/flutter_test.dart';

class _Canvas implements Canvas {
  final calls = <Invocation>[];

  @override
  void noSuchMethod(Invocation invocation) {
    calls.add(invocation);
  }
}

void main() {
  test(
    'omitted directional sides do not paint but an explicit hairline does',
    () {
      const absent = BorderDirectional();
      expect([
        absent.start,
        absent.end,
        absent.top,
        absent.bottom,
      ], List.filled(4, BorderSide.none));
      expect(absent.dimensions, EdgeInsetsDirectional.zero);
      for (final direction in TextDirection.values) {
        final canvas = _Canvas();
        final rect = Rect.fromLTWH(0, 0, 20, 10);
        absent.paint(canvas, rect, textDirection: direction);
        expect(canvas.calls, isEmpty);
        const BorderDirectional(start: BorderSide(width: 0))
            .paint(canvas, rect, textDirection: direction);
        final call = canvas.calls.single;
        expect(call.memberName, #drawPath);
        final path = call.positionalArguments[0] as Path;
        final paint = call.positionalArguments[1] as Paint;
        final x = direction == TextDirection.ltr ? 0.0 : 20.0;
        expect(path.getBounds(), Rect.fromLTRB(x, 0, x, 10));
        expect(paint.style, PaintingStyle.stroke);
        expect(paint.strokeWidth, 0.0);
      }
    },
  );
}
