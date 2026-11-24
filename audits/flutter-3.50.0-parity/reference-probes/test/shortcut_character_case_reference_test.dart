import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('character activator distinguishes case', (tester) async {
    final events = <KeyEvent>[];
    await tester.pumpWidget(
      Focus(
        autofocus: true,
        onKeyEvent: (node, event) {
          events.add(event);
          return KeyEventResult.ignored;
        },
        child: const SizedBox(),
      ),
    );
    const lower = CharacterActivator('a');
    const upper = CharacterActivator('A');
    await tester.sendKeyDownEvent(LogicalKeyboardKey.keyA, character: 'a');
    final unshifted = [
      ShortcutActivator.isActivatedBy(lower, events.last),
      ShortcutActivator.isActivatedBy(upper, events.last),
    ];
    await tester.sendKeyUpEvent(LogicalKeyboardKey.keyA);
    await tester.sendKeyDownEvent(LogicalKeyboardKey.shiftLeft);
    await tester.sendKeyDownEvent(LogicalKeyboardKey.keyA, character: 'A');
    final shifted = [
      ShortcutActivator.isActivatedBy(upper, events.last),
      ShortcutActivator.isActivatedBy(lower, events.last),
    ];
    await tester.sendKeyUpEvent(LogicalKeyboardKey.keyA);
    await tester.sendKeyUpEvent(LogicalKeyboardKey.shiftLeft);
    expect(
      [unshifted[0], shifted[0], shifted[1], unshifted[1]],
      [true, true, false, false],
    );
  });
}
