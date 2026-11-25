import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('AnimatedSize includes the aligned offset in its baseline', (
    tester,
  ) async {
    const child = ValueKey('child');
    Widget frame(bool large) => Directionality(
      textDirection: TextDirection.ltr,
      child: Center(
        child: AnimatedSize(
          duration: const Duration(milliseconds: 200),
          alignment: Alignment.bottomLeft,
          child: SizedBox(
            key: child,
            width: 100,
            height: large ? 80 : 20,
            child: large ? const Text('X') : null,
          ),
        ),
      ),
    );

    double? baseline(Finder finder) {
      final old = RenderObject.debugCheckingIntrinsics;
      RenderObject.debugCheckingIntrinsics = true;
      try {
        return tester
            .renderObject<RenderBox>(finder)
            .getDistanceToBaseline(TextBaseline.alphabetic, onlyReal: true);
      } finally {
        RenderObject.debugCheckingIntrinsics = old;
      }
    }

    final animated = find.byType(AnimatedSize);
    final text = find.byKey(child);
    await tester.pumpWidget(frame(false));
    expect(baseline(animated), isNull);
    await tester.pumpWidget(frame(true));
    await tester.pump(const Duration(milliseconds: 100));
    expect(tester.getSize(animated), const Size(100, 50));
    final dy = tester.getTopLeft(text).dy - tester.getTopLeft(animated).dy;
    expect(dy, -30);
    expect(baseline(animated), baseline(text)! + dy);
  });
}
