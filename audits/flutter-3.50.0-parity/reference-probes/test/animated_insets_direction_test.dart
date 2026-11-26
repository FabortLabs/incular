import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  for (final variant in ['padding', 'container padding', 'container margin']) {
    testWidgets('direction changes do not restart $variant', (tester) async {
      final view = GlobalKey();
      final leaf = GlobalKey();
      final child = SizedBox.expand(key: leaf);
      const duration = Duration(milliseconds: 200);
      const initial = EdgeInsets.fromLTRB(10, 0, 2, 0);
      const target = EdgeInsetsDirectional.fromSTEB(30, 0, 6, 0);
      var ends = 0;
      void onEnd() => ends++;
      Future<void> build(TextDirection direction, EdgeInsetsGeometry padding) {
        final widget = switch (variant) {
          'padding' => AnimatedPadding(
            padding: padding,
            duration: duration,
            onEnd: onEnd,
            child: child,
          ),
          'container padding' => AnimatedContainer(
            padding: padding,
            duration: duration,
            onEnd: onEnd,
            child: child,
          ),
          _ => AnimatedContainer(
            margin: padding,
            duration: duration,
            onEnd: onEnd,
            child: child,
          ),
        };
        return tester.pumpWidget(
          Directionality(
            textDirection: direction,
            child: Center(
              child: SizedBox(key: view, width: 200, height: 60, child: widget),
            ),
          ),
        );
      }

      Element? element;
      void verify(double left, double right) {
        final finder = find.byKey(leaf);
        element ??= tester.element(finder);
        expect(tester.element(finder), same(element));
        expect(
          tester.getTopLeft(finder) - tester.getTopLeft(find.byKey(view)),
          Offset(left, 0),
        );
        expect(tester.getSize(finder), Size(200 - left - right, 60));
      }

      await build(TextDirection.ltr, initial);
      verify(10, 2);
      await build(TextDirection.ltr, target);
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 50));
      verify(15, 3);
      await build(TextDirection.rtl, target);
      verify(9, 9);
      await tester.pump(const Duration(milliseconds: 50));
      verify(8, 16);
      await tester.pump(const Duration(milliseconds: 100));
      verify(6, 30);
      expect(ends, 0);
      await tester.pump(const Duration(milliseconds: 1));
      expect(ends, 1);
      await build(TextDirection.ltr, target);
      verify(30, 6);
      await tester.pump(const Duration(milliseconds: 249));
      expect(ends, 1);
    });
  }
  testWidgets('overshooting padding clamps before resolving either side', (
    tester,
  ) async {
    final view = GlobalKey();
    final leaf = GlobalKey();
    final child = SizedBox.expand(key: leaf);
    Future<void> build(EdgeInsetsGeometry padding) => tester.pumpWidget(
      Directionality(
        textDirection: TextDirection.ltr,
        child: Center(
          child: SizedBox(
            key: view,
            width: 200,
            height: 60,
            child: AnimatedPadding(
              padding: padding,
              duration: const Duration(milliseconds: 200),
              curve: const Cubic(1 / 3, -1, 2 / 3, -1),
              child: child,
            ),
          ),
        ),
      ),
    );
    await build(const EdgeInsets.fromLTRB(10, 0, 2, 0));
    await build(const EdgeInsetsDirectional.fromSTEB(30, 0, 6, 0));
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 100));
    final finder = find.byKey(leaf);
    final left =
        (tester.getTopLeft(finder) - tester.getTopLeft(find.byKey(view))).dx;
    expect(left, closeTo(16.25, 0.001));
    expect(200 - left - tester.getSize(finder).width, closeTo(3.25, 0.001));
  });
}
