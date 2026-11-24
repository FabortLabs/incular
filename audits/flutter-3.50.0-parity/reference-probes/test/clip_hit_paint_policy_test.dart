import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

class CountingRect extends CustomClipper<Rect> {
  CountingRect(this.resolved);
  final VoidCallback resolved;

  @override
  Rect getClip(Size size) {
    resolved();
    return const Rect.fromLTWH(10, 10, 30, 30);
  }

  @override
  bool shouldReclip(CountingRect oldClipper) => false;
}

void main() {
  testWidgets('custom rect hit shape is retained when painting clip is disabled', (
    tester,
  ) async {
    final log = <String>[];
    Widget scene(Clip clip) => ClipRect(
      clipBehavior: clip,
      clipper: CountingRect(() => log.add('clip')),
      child: GestureDetector(
        behavior: HitTestBehavior.opaque,
        onTap: () => log.add('tap'),
        child: const SizedBox.expand(),
      ),
    );
    await tester.pumpWidget(scene(Clip.none));
    expect(log, isEmpty);
    for (final point in const [
      Offset(9, 20), Offset(10, 10), Offset(39, 39), Offset(40, 39), Offset(39, 40),
    ]) {
      await tester.tapAt(point);
    }
    expect(log, ['clip', 'tap', 'tap']);
    await tester.pumpWidget(scene(Clip.hardEdge));
    await tester.pumpWidget(scene(Clip.none));
    await tester.tapAt(const Offset(20, 20));
    expect(log, ['clip', 'tap', 'tap', 'tap']);
  });

  testWidgets('Clip.none oval still limits pointer hits to the ellipse', (
    tester,
  ) async {
    var taps = 0;
    await tester.pumpWidget(
      ClipOval(
        clipBehavior: Clip.none,
        child: GestureDetector(
          behavior: HitTestBehavior.opaque,
          onTap: () => taps++,
          child: const SizedBox.expand(),
        ),
      ),
    );
    await tester.tapAt(const Offset(10, 10));
    expect(taps, 0);
    await tester.tapAt(tester.getCenter(find.byType(ClipOval)));
    expect(taps, 1);
  });
}
