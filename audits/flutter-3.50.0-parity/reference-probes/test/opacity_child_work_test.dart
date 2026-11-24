import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

class _CountingPainter extends CustomPainter {
  _CountingPainter(this.calls);
  final List<Size> calls;

  @override
  void paint(Canvas canvas, Size size) {
    calls.add(size);
    canvas.drawRect(
      Offset.zero & size,
      Paint()..color = const Color(0xffffffff),
    );
  }

  @override
  bool shouldRepaint(_CountingPainter oldDelegate) => true;
}

void main() {
  testWidgets('initial custom paint only runs for nonzero effective opacity', (
    tester,
  ) async {
    for (final (alpha, keepSemantics, callsExpected) in [
      (1.0, false, 1),
      (0.1, false, 1),
      (0.0, false, 0),
      (0.0, true, 0),
      (0.001, false, 0),
      (0.003, false, 1),
    ]) {
      final calls = <Size>[];
      await tester.pumpWidget(
        Center(
          key: UniqueKey(),
          child: Opacity(
            opacity: alpha,
            alwaysIncludeSemantics: keepSemantics,
            child: CustomPaint(
              size: const Size(40, 20),
              painter: _CountingPainter(calls),
            ),
          ),
        ),
      );
      expect(
        calls,
        List.filled(callsExpected, const Size(40, 20)),
        reason: 'alpha=$alpha keepSemantics=$keepSemantics',
      );
    }
  });
}
