import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets(
    'typed and legacy handlers both run before bubbling from the node',
    (tester) async {
      final node = FocusNode();
      addTearDown(node.dispose);
      final log = <String>[];
      var rawResult = KeyEventResult.handled;
      await tester.pumpWidget(
        Focus(
          onKeyEvent: (_, event) {
            if (event is KeyDownEvent) log.add('parent');
            return KeyEventResult.ignored;
          },
          child: Focus(
            focusNode: node,
            onKeyEvent: (_, event) {
              if (event is KeyDownEvent) log.add('typed');
              return KeyEventResult.skipRemainingHandlers;
            },
            onKey: (_, event) {
              if (event is RawKeyDownEvent) {
                expect(event.logicalKey, LogicalKeyboardKey.enter);
                expect(event.physicalKey, PhysicalKeyboardKey.enter);
                log.add('raw');
              }
              return rawResult;
            },
            child: const SizedBox.expand(),
          ),
        ),
      );
      node.requestFocus();
      await tester.pump();
      expect(await tester.sendKeyEvent(LogicalKeyboardKey.enter), isTrue);
      expect(log, ['typed', 'raw']);

      log.clear();
      rawResult = KeyEventResult.ignored;
      expect(await tester.sendKeyEvent(LogicalKeyboardKey.enter), isFalse);
      expect(log, ['typed', 'raw']);
    },
  );
}
