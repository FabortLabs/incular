import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets(
    'scroll line modifier follows the native platform',
    (WidgetTester tester) async {
      final controller = ScrollController();
      addTearDown(controller.dispose);
      const child = ValueKey<String>('first');
      await tester.pumpWidget(
        TestWidgetsApp(
          home: CustomScrollView(
            controller: controller,
            slivers: List<Widget>.generate(
              20,
              (int index) => SliverToBoxAdapter(
                child: Focus(
                  autofocus: index == 0,
                  child: SizedBox(key: index == 0 ? child : null, height: 50),
                ),
              ),
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();
      final finder = find.byKey(child, skipOffstage: false);
      void check(double offset) {
        expect(controller.offset, offset);
        expect(tester.getRect(finder), Rect.fromLTWH(0, -offset, 800, 50));
      }

      Future<void> chord(
        LogicalKeyboardKey modifier,
        LogicalKeyboardKey key,
      ) async {
        await tester.sendKeyDownEvent(modifier);
        await tester.sendKeyEvent(key);
        await tester.sendKeyUpEvent(modifier);
        await tester.pumpAndSettle();
      }

      final bool apple =
          defaultTargetPlatform == TargetPlatform.macOS ||
          defaultTargetPlatform == TargetPlatform.iOS;
      final matching = apple
          ? LogicalKeyboardKey.metaLeft
          : LogicalKeyboardKey.controlLeft;
      final other = apple
          ? LogicalKeyboardKey.controlLeft
          : LogicalKeyboardKey.metaLeft;
      check(0);
      await chord(other, LogicalKeyboardKey.arrowDown);
      check(0);
      await chord(matching, LogicalKeyboardKey.arrowDown);
      check(50);
      await chord(matching, LogicalKeyboardKey.arrowUp);
      check(0);
      await tester.sendKeyEvent(LogicalKeyboardKey.pageDown);
      await tester.pumpAndSettle();
      check(400);
      await tester.sendKeyEvent(LogicalKeyboardKey.pageUp);
      await tester.pumpAndSettle();
      check(0);
    },
    variant: const TargetPlatformVariant(<TargetPlatform>{
      TargetPlatform.iOS,
      TargetPlatform.macOS,
      TargetPlatform.linux,
    }),
  );
}
