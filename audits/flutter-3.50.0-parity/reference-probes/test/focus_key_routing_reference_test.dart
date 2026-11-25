import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets(
    'replacing a Focus handler preserves focus and all three outcomes',
    (tester) async {
      final outer = FocusNode();
      final inner = FocusNode(skipTraversal: true);
      addTearDown(outer.dispose);
      addTearDown(inner.dispose);
      final route = <String>[];

      Widget scene(KeyEventResult result) => Focus(
        focusNode: outer,
        onKeyEvent: (node, event) {
          if (event is KeyDownEvent) {
            expect(node, same(outer));
            route.add('outer');
            return KeyEventResult.handled;
          }
          return KeyEventResult.ignored;
        },
        child: Focus(
          focusNode: inner,
          onKeyEvent: (node, event) {
            if (event is KeyDownEvent) {
              expect(node, same(inner));
              route.add('inner');
              return result;
            }
            return KeyEventResult.ignored;
          },
          child: const SizedBox.shrink(),
        ),
      );

      await tester.pumpWidget(scene(KeyEventResult.ignored));
      inner.requestFocus();
      await tester.pump();
      for (final (result, handled, path) in [
        (KeyEventResult.ignored, true, ['inner', 'outer']),
        (KeyEventResult.handled, true, ['inner']),
        (KeyEventResult.skipRemainingHandlers, false, ['inner']),
      ]) {
        await tester.pumpWidget(scene(result));
        expect(inner.hasPrimaryFocus, isTrue);
        expect(
          await tester.sendKeyDownEvent(LogicalKeyboardKey.enter),
          handled,
        );
        await tester.sendKeyUpEvent(LogicalKeyboardKey.enter);
        expect(route, path);
        route.clear();
      }
    },
  );
}
