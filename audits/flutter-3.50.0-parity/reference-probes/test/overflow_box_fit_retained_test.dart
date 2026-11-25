import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('Default and retained OverflowBox fits preserve child geometry', (
    WidgetTester tester,
  ) async {
    const boxKey = ValueKey<String>('overflow');
    const childKey = ValueKey<String>('child');

    Widget scene(OverflowBoxFit? fit, bool hasChild) {
      final child = hasChild
          ? const SizedBox(key: childKey, width: 20, height: 10)
          : null;
      return Directionality(
        textDirection: TextDirection.ltr,
        child: Align(
          alignment: Alignment.topLeft,
          child: ConstrainedBox(
            constraints: const BoxConstraints(maxWidth: 100, maxHeight: 80),
            child: fit == null
                ? OverflowBox(key: boxKey, child: child)
                : OverflowBox(key: boxKey, fit: fit, child: child),
          ),
        ),
      );
    }

    final box = find.byKey(boxKey);
    final child = find.byKey(childKey);
    RenderConstrainedOverflowBox? retained;
    for (final (fit, hasChild, size, offset) in [
      (null, true, const Size(100, 80), const Offset(40, 35)),
      (OverflowBoxFit.deferToChild, true, const Size(20, 10), Offset.zero),
      (OverflowBoxFit.max, false, const Size(100, 80), null),
      (OverflowBoxFit.deferToChild, false, Size.zero, null),
    ]) {
      await tester.pumpWidget(scene(fit, hasChild));
      final render = tester.renderObject<RenderConstrainedOverflowBox>(box);
      retained ??= render;
      expect(render, same(retained));
      expect(tester.getSize(box), size);
      if (hasChild) {
        expect(tester.getSize(child), const Size(20, 10));
        expect(tester.getTopLeft(child) - tester.getTopLeft(box), offset);
      } else {
        expect(child, findsNothing);
        expect(render.child, isNull);
      }
    }
  });

  testWidgets('OverflowBox reports a positioned real baseline', (
    WidgetTester tester,
  ) async {
    const key = ValueKey<String>('overflow');
    Widget scene(bool hasChild) => Directionality(
      textDirection: TextDirection.ltr,
      child: Align(
        alignment: Alignment.topLeft,
        child: ConstrainedBox(
          constraints: const BoxConstraints(maxWidth: 100, maxHeight: 80),
          child: OverflowBox(
            key: key,
            maxHeight: 150,
            child: hasChild
                ? const SizedBox(
                    width: 20,
                    height: 100,
                    child: Baseline(
                      baseline: 40,
                      baselineType: TextBaseline.alphabetic,
                      child: Text('A', style: TextStyle(fontSize: 4)),
                    ),
                  )
                : null,
          ),
        ),
      ),
    );

    await tester.pumpWidget(scene(true));
    final box = tester.renderObject<RenderConstrainedOverflowBox>(
      find.byKey(key),
    );
    expect(box.getDryBaseline(box.constraints, TextBaseline.alphabetic), 30);
    await tester.pumpWidget(scene(false));
    expect(tester.renderObject(find.byKey(key)), same(box));
    expect(
      box.getDryBaseline(box.constraints, TextBaseline.alphabetic),
      isNull,
    );
  });
}
