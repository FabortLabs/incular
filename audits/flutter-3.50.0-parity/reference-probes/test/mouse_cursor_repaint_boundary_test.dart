import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

class CountingPainter extends CustomPainter {
  const CountingPainter(this.callback);
  final VoidCallback callback;

  @override
  void paint(Canvas canvas, Size size) => callback();

  @override
  bool shouldRepaint(CountingPainter oldDelegate) => false;
}

void main() {
  testWidgets(
    'cursor change repaints its shared layer, not independent layers',
    (tester) async {
      final counts = [0, 0, 0];
      Widget painter(int i) => CustomPaint(
        size: const Size(10, 10),
        painter: CountingPainter(() => counts[i]++),
      );
      Widget scene(MouseCursor cursor) => Directionality(
        textDirection: TextDirection.ltr,
        child: Align(
          alignment: Alignment.topLeft,
          child: RepaintBoundary(
            child: Column(
              children: [
                MouseRegion(cursor: cursor, child: painter(0)),
                painter(1),
                RepaintBoundary(child: painter(2)),
              ],
            ),
          ),
        ),
      );
      await tester.pumpWidget(scene(SystemMouseCursors.forbidden));
      expect(counts, [1, 1, 1]);
      counts.fillRange(0, 3, 0);
      await tester.pumpWidget(scene(SystemMouseCursors.text));
      expect(counts, [1, 1, 0]);
      counts.fillRange(0, 3, 0);
      await tester.pumpWidget(scene(SystemMouseCursors.text));
      expect(counts, [0, 0, 0]);
    },
  );
}
