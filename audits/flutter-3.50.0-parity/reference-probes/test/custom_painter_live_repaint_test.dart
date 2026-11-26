import 'package:flutter/semantics.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

class _WatchingPainter extends CustomPainter {
  _WatchingPainter(this.color, this.paints, this.regions)
    : super(repaint: color);

  final ValueNotifier<Color> color;
  final List<Color> paints;
  final List<Size> regions;

  @override
  void paint(Canvas canvas, Size size) {
    paints.add(color.value);
    canvas.drawRect(Offset.zero & size, Paint()..color = color.value);
  }

  @override
  bool shouldRepaint(_WatchingPainter oldDelegate) => false;

  @override
  SemanticsBuilderCallback get semanticsBuilder => (size) {
    regions.add(size);
    return [
      const CustomPainterSemantics(
        rect: Rect.fromLTWH(0, 0, 10, 10),
        properties: SemanticsProperties(
          label: 'status',
          textDirection: TextDirection.ltr,
        ),
      ),
    ];
  };
}

void main() {
  testWidgets(
    'CustomPainter listenable repaints without rebuilding semantics',
    (tester) async {
      final semantics = tester.ensureSemantics();
      const a = Color(0xFFC80000);
      const b = Color(0xFF0000C8);
      const c = Color(0xFF00C800);
      final first = ValueNotifier<Color>(a);
      final second = ValueNotifier<Color>(b);
      addTearDown(first.dispose);
      addTearDown(second.dispose);
      final paints = <Color>[];
      final regions = <Size>[];
      Future<void> show(ValueNotifier<Color> color) => tester.pumpWidget(
        Align(
          alignment: Alignment.topLeft,
          child: RepaintBoundary(
            child: CustomPaint(
              size: const Size(100, 60),
              painter: _WatchingPainter(color, paints, regions),
            ),
          ),
        ),
      );
      await show(first);
      expect(paints, [a]);
      expect(regions, [const Size(100, 60)]);
      first.value = b;
      await tester.pump();
      expect(paints, [a, b]);
      expect(regions, [const Size(100, 60)]);

      await show(second);
      final count = paints.length;
      first.value = c;
      await tester.pump();
      expect(paints.length, count);
      second.value = c;
      await tester.pump();
      expect(paints.last, c);
      expect(paints.length, count + 1);
      semantics.dispose();
    },
  );
}
