import 'package:flutter/gestures.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  for (final (strategy, added) in [
    (MultitouchDragStrategy.latestPointer, 20.0),
    (MultitouchDragStrategy.sumAllPointers, 50.0),
    (MultitouchDragStrategy.averageBoundaryPointers, 30.0),
  ]) {
    testWidgets('retained list uses ${strategy.name} for two contacts', (
      tester,
    ) async {
      final controller = ScrollController(initialScrollOffset: 100);
      addTearDown(controller.dispose);
      final lifecycle = <String>[];
      await tester.pumpWidget(
        MediaQuery(
          data: const MediaQueryData(),
          child: Directionality(
            textDirection: TextDirection.ltr,
            child: Align(
              alignment: Alignment.topLeft,
              child: SizedBox(
                width: 100,
                height: 100,
                child: ScrollConfiguration(
                  behavior: const ScrollBehavior().copyWith(
                    multitouchDragStrategy: strategy,
                    scrollbars: false,
                  ),
                  child: NotificationListener<ScrollNotification>(
                    onNotification: (notification) {
                      if (notification is ScrollStartNotification) {
                        lifecycle.add('start');
                      } else if (notification is ScrollEndNotification) {
                        lifecycle.add('end');
                      }
                      return false;
                    },
                    child: ListView.builder(
                      controller: controller,
                      dragStartBehavior: DragStartBehavior.down,
                      physics: const ClampingScrollPhysics(),
                      itemCount: 20,
                      itemExtent: 40,
                      itemBuilder: (_, index) => Text('$index'),
                    ),
                  ),
                ),
              ),
            ),
          ),
        ),
      );
      expect(controller.offset, 100);
      final first = await tester.startGesture(const Offset(45, 80), pointer: 5);
      await first.moveBy(const Offset(0, -40));
      expect(controller.offset, 140);
      await tester.pump(const Duration(milliseconds: 16));
      final second = await tester.startGesture(
        const Offset(55, 80),
        pointer: 6,
      );
      await first.moveBy(const Offset(0, -30));
      await second.moveBy(const Offset(0, -20));
      expect(controller.offset, 140 + added);
      expect(lifecycle, ['start']);
      await first.cancel();
      expect(lifecycle, ['start']);
      await tester.pump(const Duration(milliseconds: 16));
      await second.moveBy(const Offset(0, -10));
      expect(controller.offset, 150 + added);
      await second.cancel();
      expect(lifecycle, ['start', 'end']);
    });
  }
}
