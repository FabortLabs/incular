import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('modern focus key phases and non-consuming skip', (tester) async {
    final node = FocusNode();
    final log = <int>[];
    var early = KeyEventResult.ignored;
    var focus = KeyEventResult.ignored;
    var late = KeyEventResult.ignored;
    final manager = tester.binding.focusManager;
    KeyEventResult earlyHandler(KeyEvent e) {
      log.add(1);
      return e is KeyDownEvent ? early : KeyEventResult.ignored;
    }

    KeyEventResult lateHandler(KeyEvent e) {
      log.add(3);
      return e is KeyDownEvent ? late : KeyEventResult.ignored;
    }

    await tester.pumpWidget(
      Focus(
        focusNode: node,
        onKeyEvent: (n, e) {
          log.add(2);
          return e is KeyDownEvent ? focus : KeyEventResult.ignored;
        },
        child: const SizedBox(),
      ),
    );
    addTearDown(node.dispose);
    node.requestFocus();
    await tester.pump();
    expect(node.hasPrimaryFocus, isTrue);
    const key = LogicalKeyboardKey.digit1;
    Future<void> dispatch(bool handled, List<int> down, List<int> up) async {
      expect(await simulateKeyDownEvent(key), handled);
      expect(log, down);
      log.clear();
      expect(await simulateKeyUpEvent(key), isFalse);
      expect(log, up);
      log.clear();
    }

    await dispatch(false, [2], [2]);
    manager.addEarlyKeyEventHandler(earlyHandler);
    manager.addLateKeyEventHandler(lateHandler);
    addTearDown(() {
      manager.removeEarlyKeyEventHandler(earlyHandler);
      manager.removeLateKeyEventHandler(lateHandler);
    });
    early = KeyEventResult.handled;
    await dispatch(true, [1], [1, 2, 3]);
    early = KeyEventResult.ignored;
    late = KeyEventResult.handled;
    await dispatch(true, [1, 2, 3], [1, 2, 3]);
    focus = KeyEventResult.handled;
    await dispatch(true, [1, 2], [1, 2, 3]);
    early = KeyEventResult.skipRemainingHandlers;
    focus = KeyEventResult.skipRemainingHandlers;
    await dispatch(true, [1, 2, 3], [1, 2, 3]);
    late = KeyEventResult.skipRemainingHandlers;
    await dispatch(false, [1, 2, 3], [1, 2, 3]);
    manager.removeEarlyKeyEventHandler(earlyHandler);
    manager.removeLateKeyEventHandler(lateHandler);
    focus = KeyEventResult.ignored;
    await dispatch(false, [2], [2]);
  });
}
