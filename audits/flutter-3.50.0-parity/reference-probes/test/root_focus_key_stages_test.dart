import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('focus key stages run when only the root scope owns focus', (
    tester,
  ) async {
    await tester.pumpWidget(const SizedBox.expand());
    final manager = FocusManager.instance;
    manager.rootScope.requestFocus();
    await tester.pump();
    expect(manager.primaryFocus, same(manager.rootScope));

    final log = <String>[];
    var early = KeyEventResult.handled;
    var late = KeyEventResult.handled;
    KeyEventResult earlyHandler(KeyEvent event) {
      if (event is! KeyDownEvent) return KeyEventResult.ignored;
      log.add('early');
      return early;
    }

    KeyEventResult lateHandler(KeyEvent event) {
      if (event is! KeyDownEvent) return KeyEventResult.ignored;
      log.add('late');
      return late;
    }

    manager.addEarlyKeyEventHandler(earlyHandler);
    manager.addLateKeyEventHandler(lateHandler);
    addTearDown(() {
      manager.removeEarlyKeyEventHandler(earlyHandler);
      manager.removeLateKeyEventHandler(lateHandler);
    });

    expect(await tester.sendKeyEvent(LogicalKeyboardKey.digit1), isTrue);
    expect(log, ['early']);
    log.clear();
    early = KeyEventResult.ignored;
    expect(await tester.sendKeyEvent(LogicalKeyboardKey.digit1), isTrue);
    expect(log, ['early', 'late']);
    log.clear();
    late = KeyEventResult.skipRemainingHandlers;
    expect(await tester.sendKeyEvent(LogicalKeyboardKey.digit1), isFalse);
    expect(log, ['early', 'late']);
  });
}
