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
}
