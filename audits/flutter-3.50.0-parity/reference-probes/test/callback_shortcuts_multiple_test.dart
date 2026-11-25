import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('matching shortcut bindings run in order after replacement', (
    tester,
  ) async {
    final seen = <int>[];
    const first = SingleActivator(LogicalKeyboardKey.keyA);
    const second = SingleActivator(
      LogicalKeyboardKey.keyA,
      includeRepeats: false,
    );
    final bindings = <ShortcutActivator, VoidCallback>{
      first: () => seen.add(1),
      second: () => seen.add(2),
    };
    bindings[first] = () => seen.add(3);
    await tester.pumpWidget(
      CallbackShortcuts(
        bindings: bindings,
        child: const Focus(autofocus: true, child: SizedBox()),
      ),
    );
    await tester.pump();
    await tester.sendKeyDownEvent(LogicalKeyboardKey.keyA);
    expect(seen, [3, 2]);
    await tester.sendKeyUpEvent(LogicalKeyboardKey.keyA);
  });
}
