import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets(
    'building and discarding a Focus descriptor cannot change its mounted node',
    (tester) async {
      final node = FocusNode();
      addTearDown(node.dispose);
      var enters = 0;
      KeyEventResult onKeyEvent(FocusNode called, KeyEvent event) {
        expect(called, same(node));
        if (event is KeyDownEvent &&
            event.logicalKey == LogicalKeyboardKey.enter) {
          enters++;
        }
        return KeyEventResult.handled;
      }

      Focus scene(bool allowed) => Focus(
        focusNode: node,
        canRequestFocus: allowed,
        skipTraversal: allowed,
        descendantsAreFocusable: allowed,
        onKeyEvent: onKeyEvent,
        child: const SizedBox.expand(),
      );
      await tester.pumpWidget(scene(true));
      node.requestFocus();
      await tester.pump();
      expect(node.hasPrimaryFocus, isTrue);

      final discarded = scene(false);
      expect(discarded.canRequestFocus, isFalse);
      expect(node.canRequestFocus, isTrue);
      expect(node.skipTraversal, isTrue);
      expect(node.descendantsAreFocusable, isTrue);
      await tester.pump();
      expect(node.hasPrimaryFocus, isTrue);
      await tester.sendKeyEvent(LogicalKeyboardKey.enter);
      expect(enters, 1);

      await tester.pumpWidget(scene(false));
      await tester.pump();
      expect(node.canRequestFocus, isFalse);
      expect(node.skipTraversal, isFalse);
      expect(node.descendantsAreFocusable, isFalse);
      expect(node.hasPrimaryFocus, isFalse);
      await tester.sendKeyEvent(LogicalKeyboardKey.enter);
      expect(enters, 1);
    },
  );
}
