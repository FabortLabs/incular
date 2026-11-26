import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

const caller = ValueKey<String>('intent caller');

Widget list(ScrollController controller, {bool reverse = false}) => ListView(
  controller: controller,
  reverse: reverse,
  children: const [
    SizedBox(key: caller, height: 600, child: Text('one')),
    SizedBox(height: 600, child: Text('two')),
    SizedBox(height: 600, child: Text('three')),
  ],
);

Widget actions(Widget child) => Directionality(
  textDirection: TextDirection.ltr,
  child: Actions(
    actions: <Type, Action<Intent>>{ScrollIntent: ScrollAction()},
    child: child,
  ),
);

Future<void> invoke(
  WidgetTester tester,
  Finder source,
  AxisDirection direction,
) async {
  Actions.invoke(tester.element(source), ScrollIntent(direction: direction));
  await tester.pump();
  await tester.pump(const Duration(milliseconds: 150));
}

void main() {
  testWidgets(
    'ScrollIntent resolves an inherited primary controller from a sibling',
    (tester) async {
      final controller = ScrollController();
      addTearDown(controller.dispose);
      const outside = ValueKey<String>('outside caller');
      await tester.pumpWidget(
        actions(
          PrimaryScrollController(
            controller: controller,
            child: Column(
              children: [
                const SizedBox(key: outside),
                Expanded(child: list(controller)),
              ],
            ),
          ),
        ),
      );
      await invoke(tester, find.byKey(outside), AxisDirection.down);
      expect(controller.offset, 50.0);
    },
  );

  testWidgets('ScrollIntent uses physical direction on a reversed viewport', (
    tester,
  ) async {
    final controller = ScrollController();
    addTearDown(controller.dispose);
    await tester.pumpWidget(actions(list(controller, reverse: true)));
    for (final (direction, expected) in [
      (AxisDirection.up, 50.0),
      (AxisDirection.left, 50.0),
      (AxisDirection.down, 0.0),
    ]) {
      await invoke(tester, find.byKey(caller), direction);
      expect(controller.offset, expected);
    }
  });

  testWidgets(
    'Scrollable calculates custom line and page increments from live metrics',
    (tester) async {
      final controller = ScrollController();
      final observed = <ScrollIncrementDetails>[];
      final pixels = <double>[];
      addTearDown(controller.dispose);
      await tester.pumpWidget(
        actions(
          Scrollable(
            controller: controller,
            incrementCalculator: (details) {
              observed.add(details);
              pixels.add(details.metrics.pixels);
              return details.type == ScrollIncrementType.line
                  ? 37
                  : details.metrics.viewportDimension / 3;
            },
            viewportBuilder: (context, offset) => Viewport(
              offset: offset,
              slivers: const [
                SliverToBoxAdapter(child: SizedBox(key: caller, height: 1800)),
              ],
            ),
          ),
        ),
      );
      final origin = tester.element(find.byKey(caller));
      Actions.invoke(origin, const ScrollIntent(direction: AxisDirection.down));
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 150));
      expect(controller.offset, 37.0);
      Actions.invoke(
        origin,
        const ScrollIntent(
          direction: AxisDirection.down,
          type: ScrollIncrementType.page,
        ),
      );
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 150));
      expect(controller.offset, 237.0);
      await invoke(tester, find.byKey(caller), AxisDirection.left);
      expect(controller.offset, 237.0);
      expect(observed.map((details) => details.type), [
        ScrollIncrementType.line,
        ScrollIncrementType.page,
      ]);
      expect(pixels, [0.0, 37.0]);
      // ScrollIncrementDetails holds the live position, so read these
      // stable dimensions after movement rather than treating it as a copy.
      for (final details in observed) {
        expect(details.metrics.viewportDimension, 600.0);
        expect(details.metrics.minScrollExtent, 0.0);
        expect(details.metrics.maxScrollExtent, 1200.0);
      }
    },
  );
}
