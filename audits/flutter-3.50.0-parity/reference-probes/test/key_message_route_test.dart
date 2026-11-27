// Authored for Incular parity; not an upstream test registration.
// ignore_for_file: deprecated_member_use

import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('global delivery precedes the replaceable key message route', (
    WidgetTester tester,
  ) async {
    final manager = tester.binding.keyEventManager;
    final original = manager.keyMessageHandler;
    final calls = <String>[];
    var hardwareHandles = true;
    bool hardware(KeyEvent event) {
      calls.add('hardware');
      return hardwareHandles;
    }

    HardwareKeyboard.instance.addHandler(hardware);
    addTearDown(() {
      manager.keyMessageHandler = original;
      HardwareKeyboard.instance.removeHandler(hardware);
    });
    await tester.pumpWidget(
      Focus(
        autofocus: true,
        onKeyEvent: (_, _) {
          calls.add('focus');
          return KeyEventResult.ignored;
        },
        child: const SizedBox.shrink(),
      ),
    );
    await tester.pump();
    final seen = <(bool, bool, bool)>[];
    manager.keyMessageHandler = (message) {
      calls.add('message');
      final down = message.events.single is KeyDownEvent;
      seen.add((
        down,
        message.rawEvent is RawKeyDownEvent,
        HardwareKeyboard.instance.isPhysicalKeyPressed(PhysicalKeyboardKey.keyA),
      ));
      return !down;
    };
    expect(await simulateKeyDownEvent(LogicalKeyboardKey.keyA), isTrue);
    hardwareHandles = false;
    expect(await simulateKeyUpEvent(LogicalKeyboardKey.keyA), isTrue);
    expect(await simulateKeyDownEvent(LogicalKeyboardKey.keyA), isFalse);
    expect(
      calls,
      ['hardware', 'message', 'hardware', 'message', 'hardware', 'message'],
    );
    expect(seen, [(true, true, true), (false, false, false), (true, true, true)]);

    calls.clear();
    manager.keyMessageHandler = null;
    expect(await simulateKeyUpEvent(LogicalKeyboardKey.keyA), isFalse);
    expect(calls, ['hardware']);

    calls.clear();
    manager.keyMessageHandler = original;
    expect(await simulateKeyDownEvent(LogicalKeyboardKey.keyA), isFalse);
    expect(await simulateKeyUpEvent(LogicalKeyboardKey.keyA), isFalse);
    expect(calls, ['hardware', 'focus', 'hardware', 'focus']);
  });
}
