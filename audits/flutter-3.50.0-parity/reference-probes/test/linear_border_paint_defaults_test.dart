import 'package:flutter/painting.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  const canvasRect = Rect.fromLTWH(0, 0, 100, 100);

  test('linear border scaling clears selected edges and paint', () {
    const side = BorderSide(color: Color(0x0f00ff00), width: 4);
    final border = LinearBorder.start(side: side, size: 0.5, alignment: -0.25);
    final scaled = border.scale(0.5);
    expect(scaled.side.width, 2);
    expect(scaled.start, isNull);
    expect(OutlinedBorder.lerp(null, border, 0.5), scaled);
    expect(
      LinearBorderEdge.lerp(null, border.start, 0.5),
      const LinearBorderEdge(size: 0.25, alignment: -0.25),
    );
    expect(
      (Canvas canvas) =>
          scaled.paint(canvas, canvasRect, textDirection: TextDirection.ltr),
      paintsNothing,
    );
  });

  test(
    'solid zero width is a hairline; absent and empty edges do not paint',
    () {
      final top = LinearBorder.top(side: const BorderSide(width: 0));
      expect(
        (Canvas canvas) =>
            top.paint(canvas, canvasRect, textDirection: TextDirection.ltr),
        paints..path(style: PaintingStyle.stroke, strokeWidth: 0),
      );
      expect(
        (Canvas canvas) => LinearBorder.top(
          side: const BorderSide(width: 4),
          size: 0,
        ).paint(canvas, canvasRect, textDirection: TextDirection.ltr),
        paintsNothing,
      );
    },
  );

  test(
    'out-of-range edge sizes fail instead of defaulting to visible lengths',
    () {
      LinearBorderEdge edge(double size) => LinearBorderEdge(size: size);
      expect(() => edge(-0.1), throwsAssertionError);
      expect(() => edge(1.1), throwsAssertionError);
    },
  );
}
