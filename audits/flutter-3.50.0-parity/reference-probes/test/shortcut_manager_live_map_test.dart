import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

class RunIntent extends Intent {
  const RunIntent();
}

void main() {
  testWidgets(
    'Replacing a manager map routes the next key with the same focus',
    (tester) async {
      final focus = FocusNode();
      addTearDown(focus.dispose);
      final manager = ShortcutManager(
        shortcuts: const {
          SingleActivator(LogicalKeyboardKey.keyA): RunIntent(),
        },
      );
      addTearDown(manager.dispose);
      var actions = 0;
      await tester.pumpWidget(
        Actions(
          actions: <Type, Action<Intent>>{
            RunIntent: CallbackAction<RunIntent>(onInvoke: (_) => ++actions),
          },
          child: Shortcuts.manager(
            manager: manager,
            child: Focus(
              focusNode: focus,
              autofocus: true,
              child: const SizedBox(),
            ),
          ),
        ),
      );
      await tester.pump();
      expect(focus.hasPrimaryFocus, isTrue);
      expect(await tester.sendKeyEvent(LogicalKeyboardKey.keyA), isTrue);
      expect(actions, 1);
      manager.shortcuts = const {
        SingleActivator(LogicalKeyboardKey.keyB): RunIntent(),
      };
      expect(await tester.sendKeyEvent(LogicalKeyboardKey.keyA), isFalse);
      expect(await tester.sendKeyEvent(LogicalKeyboardKey.keyB), isTrue);
      expect(actions, 2);
      expect(focus.hasPrimaryFocus, isTrue);
    },
  );

  testWidgets(
    'Modal manager stops the focus chain without consuming an unmapped key',
    (tester) async {
      final manager = ShortcutManager(modal: true);
      addTearDown(manager.dispose);
      final leaf = FocusNode();
      addTearDown(leaf.dispose);
      var outerDowns = 0;
      await tester.pumpWidget(
        Focus(
          onKeyEvent: (_, event) {
            if (event is KeyDownEvent) {
              outerDowns++;
              return KeyEventResult.handled;
            }
            return KeyEventResult.ignored;
          },
          child: Shortcuts.manager(
            manager: manager,
            child: Focus(
              focusNode: leaf,
              autofocus: true,
              child: const SizedBox(),
            ),
          ),
        ),
      );
      await tester.pump();
      expect(leaf.hasPrimaryFocus, isTrue);
      expect(await tester.sendKeyEvent(LogicalKeyboardKey.keyA), isFalse);
      expect(outerDowns, 0);
    },
  );
}
