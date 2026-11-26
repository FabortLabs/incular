import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets(
    'one sliver decoration paints in front or behind the same child',
    (tester) async {
      const outer = ValueKey('decorated sliver');
      const child = ValueKey('child');
      const blue = Color(0xFF2196F3);
      const red = Color(0xFFF44336);
      Widget scene(DecorationPosition position) => Directionality(
        textDirection: TextDirection.ltr,
        child: Align(
          alignment: Alignment.topLeft,
          child: SizedBox(
            width: 100,
            height: 100,
            child: CustomScrollView(
              slivers: [
                DecoratedSliver(
                  key: outer,
                  decoration: const BoxDecoration(color: blue),
                  position: position,
                  sliver: const SliverToBoxAdapter(
                    child: ColoredBox(
                      key: child,
                      color: red,
                      child: SizedBox(height: 100),
                    ),
                  ),
                ),
              ],
            ),
          ),
        ),
      );
      const rect = Rect.fromLTWH(0, 0, 100, 100);
      await tester.pumpWidget(scene(DecorationPosition.background));
      final childBefore = tester.element(find.byKey(child));
      expect(
        find.byKey(outer),
        paints
          ..rect(rect: rect, color: blue)
          ..rect(rect: rect, color: red),
      );
      await tester.pumpWidget(scene(DecorationPosition.foreground));
      expect(tester.element(find.byKey(child)), same(childBefore));
      expect(
        find.byKey(outer),
        paints
          ..rect(rect: rect, color: red)
          ..rect(rect: rect, color: blue),
      );
    },
  );
}
