import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

class CounterPainter extends CustomPainter {
  CounterPainter(this.onPaint);

  final VoidCallback onPaint;

  @override
  void paint(Canvas canvas, Size size) => onPaint();

  @override
  bool shouldRepaint(CounterPainter oldDelegate) => true;
}

void main() {
  testWidgets('paint is suppressed below a half alpha byte', (tester) async {
    for (final (alpha, expected) in [
      (0.0, 0),
      (0.001, 0),
      (0.0019607841968536377, 0),
      (0.0019607844296842813, 1),
      (0.003, 1),
    ]) {
      var paints = 0;
      await tester.pumpWidget(
        Opacity(
          opacity: alpha,
          alwaysIncludeSemantics: true,
          child: CustomPaint(
            size: const Size(20, 20),
            painter: CounterPainter(() => paints += 1),
          ),
        ),
      );
      expect(paints, expected, reason: 'opacity $alpha');
    }
  });
}
