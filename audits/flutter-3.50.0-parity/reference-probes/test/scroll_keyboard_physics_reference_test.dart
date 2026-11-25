import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('scroll physics gates shortcuts but not controller jumps', (
    WidgetTester tester,
  ) async {
    for (final bool never in [false, true]) {
      final controller = ScrollController();
      addTearDown(controller.dispose);
      const target = ValueKey<String>('first');
      await tester.pumpWidget(
        TestWidgetsApp(
          home: CustomScrollView(
            key: ValueKey<bool>(never),
            controller: controller,
            physics: never ? const NeverScrollableScrollPhysics() : null,
            slivers: List<Widget>.generate(
              20,
              (int index) => SliverToBoxAdapter(
                child: Focus(
                  autofocus: index == 0,
                  child: SizedBox(key: index == 0 ? target : null, height: 50),
                ),
              ),
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();
      final finder = find.byKey(target, skipOffstage: false);
      void check(double offset) {
        expect(controller.offset, offset);
        expect(tester.getRect(finder), Rect.fromLTWH(0, -offset, 800, 50));
      }

      check(0);
      await tester.sendKeyDownEvent(LogicalKeyboardKey.controlLeft);
      await tester.sendKeyEvent(LogicalKeyboardKey.arrowDown);
      await tester.sendKeyUpEvent(LogicalKeyboardKey.controlLeft);
      await tester.pumpAndSettle();
      check(never ? 0 : 50);
      await tester.sendKeyEvent(LogicalKeyboardKey.pageDown);
      await tester.pumpAndSettle();
      check(never ? 0 : 400);
      controller.jumpTo(80);
      await tester.pumpAndSettle();
      check(80);
    }
  });
}
