import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('SizedOverflowBox passes incoming constraints to its child', (
    WidgetTester tester,
  ) async {
    const Key box = ValueKey('box');
    const Key child = ValueKey('child');
    await tester.pumpWidget(
      const Directionality(
        textDirection: TextDirection.ltr,
        child: Center(
          child: SizedOverflowBox(
            key: box,
            size: Size(100, 100),
            child: SizedBox(key: child, width: 200, height: 150),
          ),
        ),
      ),
    );
    expect(
      tester.getRect(find.byKey(box)),
      const Rect.fromLTWH(350, 250, 100, 100),
    );
    expect(
      tester.getRect(find.byKey(child)),
      const Rect.fromLTWH(300, 225, 200, 150),
    );
  });

  testWidgets(
    'SizedOverflowBox reacts to inherited direction without remount',
    (WidgetTester tester) async {
      const box = ValueKey<String>('box');
      const child = ValueKey<String>('child');
      Widget build(TextDirection direction) => Directionality(
        textDirection: direction,
        child: const Center(
          child: SizedOverflowBox(
            key: box,
            size: Size(100, 100),
            alignment: AlignmentDirectional.bottomStart,
            child: SizedBox(key: child, width: 50, height: 50),
          ),
        ),
      );
      RenderSizedOverflowBox? retained;
      for (final (direction, x) in [
        (TextDirection.rtl, 400.0),
        (TextDirection.ltr, 350.0),
        (TextDirection.rtl, 400.0),
      ]) {
        await tester.pumpWidget(build(direction));
        final render = tester.renderObject<RenderSizedOverflowBox>(
          find.byKey(box),
        );
        retained ??= render;
        expect(render, same(retained));
        expect(
          tester.getRect(find.byKey(child)),
          Rect.fromLTWH(x, 300, 50, 50),
        );
      }
    },
  );
}
