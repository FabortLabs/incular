import 'package:flutter/rendering.dart' show ImageFilterLayer;
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

class _Painter extends CustomPainter {
  const _Painter(this.paintCounter);

  final VoidCallback paintCounter;

  @override
  void paint(Canvas canvas, Size size) {
    paintCounter();
    canvas.drawRect(
      Offset.zero & size,
      Paint()..color = const Color(0xffffffff),
    );
  }

  @override
  bool shouldRepaint(covariant _Painter oldDelegate) => false;
}

void main() {
  testWidgets('a sampled singular transform still records its child', (
    tester,
  ) async {
    for (final (x, quality, expected) in [
      (0.0, null, 0),
      (0.0, FilterQuality.low, 1),
      (0.25, FilterQuality.low, 1),
    ]) {
      var paints = 0;
      await tester.pumpWidget(
        Center(
          child: Transform(
            key: UniqueKey(),
            transform: Matrix4.diagonal3Values(x, 1, 1),
            filterQuality: quality,
            child: RepaintBoundary(
              child: CustomPaint(
                painter: _Painter(() => paints++),
                size: const Size(30, 20),
              ),
            ),
          ),
        ),
      );
      expect(tester.takeException(), isNull);
      expect(paints, expected, reason: '$x with $quality');
      expect(tester.layers.whereType<ImageFilterLayer>(), hasLength(expected));
    }
  });
}
