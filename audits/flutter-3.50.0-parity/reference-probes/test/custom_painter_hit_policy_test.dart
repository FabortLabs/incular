import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

class _HitPainter extends CustomPainter {
  _HitPainter(this.hit);
  final bool? Function(Offset) hit;

  @override
  bool? hitTest(Offset position) => hit(position);

  @override
  void paint(Canvas canvas, Size size) {}

  @override
  bool shouldRepaint(_HitPainter oldDelegate) => false;
}

Widget _view(Widget child) => Directionality(
  textDirection: TextDirection.ltr,
  child: Align(alignment: Alignment.topLeft, child: child),
);

Widget _tapTarget(VoidCallback clicked) => GestureDetector(
  behavior: HitTestBehavior.opaque,
  onTap: clicked,
  child: const SizedBox(width: 100, height: 100),
);

void main() {
  testWidgets('background painter defaults to hit and may decline', (
    tester,
  ) async {
    int calls = 0;
    Widget build(bool? decision) => _view(
      Stack(
        children: [
          _tapTarget(() => calls++),
          CustomPaint(
            size: const Size(100, 100),
            painter: _HitPainter((_) => decision),
          ),
        ],
      ),
    );
    await tester.pumpWidget(build(null));
    await tester.tapAt(const Offset(20, 20));
    expect(calls, 0);

    await tester.pumpWidget(build(false));
    await tester.tapAt(const Offset(20, 20));
    expect(calls, 1);
  });

  testWidgets('foreground may claim a point before its child', (tester) async {
    int calls = 0;
    Offset? seen;
    Widget build(bool? Function(Offset) hit) => _view(
      CustomPaint(
        foregroundPainter: _HitPainter(hit),
        child: _tapTarget(() => calls++),
      ),
    );
    await tester.pumpWidget(build((_) => null));
    await tester.tapAt(const Offset(20, 20));
    expect(calls, 1);

    await tester.pumpWidget(
      build((point) {
        seen = point;
        return point.dx < 40;
      }),
    );
    await tester.tapAt(const Offset(20, 20));
    expect(seen, const Offset(20, 20));
    expect(calls, 1);
    await tester.tapAt(const Offset(60, 20));
    expect(seen, const Offset(60, 20));
    expect(calls, 2);
  });
}
