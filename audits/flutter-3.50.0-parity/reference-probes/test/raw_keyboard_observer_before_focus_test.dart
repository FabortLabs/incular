import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets(
    'raw ancestor observes before the focused child handles the key',
    (tester) async {
      final rawNode = FocusNode();
      final childNode = FocusNode();
      final events = <String>[];
      addTearDown(rawNode.dispose);
      addTearDown(childNode.dispose);
      final child = Focus(
        focusNode: childNode,
        onKeyEvent: (_, event) {
          events.add(event is KeyDownEvent ? 'typed down' : 'typed up');
          return KeyEventResult.handled;
        },
        child: const SizedBox.expand(),
      );

      await tester.pumpWidget(
        RawKeyboardListener(
          focusNode: rawNode,
          onKey: (event) =>
              events.add(event is RawKeyDownEvent ? 'raw down' : 'raw up'),
          child: child,
        ),
      );
      childNode.requestFocus();
      await tester.pump();
      expect(rawNode.hasFocus, isTrue);
      expect(childNode.hasPrimaryFocus, isTrue);
      await tester.sendKeyEvent(LogicalKeyboardKey.keyA, platform: 'fuchsia');
      expect(events, ['raw down', 'typed down', 'raw up', 'typed up']);

      events.clear();
      await tester.pumpWidget(child);
      childNode.requestFocus();
      await tester.pump();
      await tester.sendKeyEvent(LogicalKeyboardKey.keyA, platform: 'fuchsia');
      expect(events, ['typed down', 'typed up']);
    },
  );
}
