import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  for (final axis in Axis.values) {
    testWidgets(
      'scroll-body child sizes to cache, then paint extent, in $axis',
      (tester) async {
        final handle = tester.ensureSemantics();
        try {
          final controller = ScrollController();
          addTearDown(controller.dispose);
          const key = ValueKey('fill');
          const label = 'Filling child';
          final vertical = axis == Axis.vertical;
          final viewportExtent = vertical ? 600.0 : 800.0;
          await tester.pumpWidget(
            Directionality(
              textDirection: TextDirection.ltr,
              child: CustomScrollView(
                controller: controller,
                scrollDirection: axis,
                cacheExtent: 250,
                slivers: [
                  SliverFixedExtentList.builder(
                    itemExtent: 150,
                    itemCount: 8,
                    itemBuilder: (_, index) => SizedBox(key: ValueKey(index)),
                  ),
                  SliverFillRemaining(
                    child: SizedBox(
                      key: key,
                      width: 100,
                      height: 100,
                      child: Semantics(label: label, container: true),
                    ),
                  ),
                ],
              ),
            ),
          );
          final finder = find.byKey(key, skipOffstage: false);
          final semantic = find.semantics.byLabel(label);
          RenderObject? retainedRender;
          for (final (delta, size, hidden) in [
            (-250.0, 0.0, null),
            (-200.0, 50.0, true),
            (0.0, 100.0, true),
            (25.0, 25.0, false),
            (50.0, 50.0, false),
          ]) {
            controller.jumpTo(1200 - viewportExtent + delta);
            await tester.pump();
            final child = tester.renderObject(finder);
            retainedRender ??= child;
            expect(child, same(retainedRender));
            final childSize = tester.getSize(finder);
            final origin = tester.getTopLeft(finder);
            expect(vertical ? childSize.height : childSize.width, size);
            expect(vertical ? origin.dy : origin.dx, viewportExtent - delta);
            if (hidden == null) {
              expect(semantic, findsNothing);
            } else {
              expect(semantic, findsOne);
              expect(semantic, isSemantics(isHidden: hidden));
            }
            expect(controller.position.maxScrollExtent, 1200);
          }
        } finally {
          handle.dispose();
        }
      },
    );
  }
}
