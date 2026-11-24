import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('hidden keyboard focus is independent of pointer policy', (
    tester,
  ) async {
    for (final policy in [0, 1, 2]) {
      final node = FocusNode();
      addTearDown(node.dispose);
      var keyDowns = 0;
      Widget scene(bool visible) {
        final child = KeyboardListener(
          focusNode: node,
          onKeyEvent: (event) {
            if (event is KeyDownEvent) {
              keyDowns++;
            }
          },
          child: const SizedBox.shrink(),
        );
        return Directionality(
          textDirection: TextDirection.ltr,
          child: policy == 2
              ? Offstage(offstage: !visible, child: child)
              : Visibility(
                  visible: visible,
                  maintainState: true,
                  maintainFocusability: policy == 1,
                  child: child,
                ),
        );
      }

      await tester.pumpWidget(scene(true));
      node.requestFocus();
      await tester.pump();
      expect(node.hasPrimaryFocus, true);
      await tester.sendKeyEvent(LogicalKeyboardKey.keyA);
      expect(keyDowns, 1);

      await tester.pumpWidget(scene(false));
      expect(node.hasPrimaryFocus, policy != 0);
      node.requestFocus();
      await tester.pump();
      expect(node.hasPrimaryFocus, policy != 0);
      await tester.sendKeyEvent(LogicalKeyboardKey.keyB);
      expect(keyDowns, policy == 0 ? 1 : 2);
      await tester.pumpWidget(const SizedBox.shrink());
    }
  });
}
