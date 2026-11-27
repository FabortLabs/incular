import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

class _Run extends Intent {
  const _Run();
}

class _RecordContext extends ContextAction<_Run> {
  BuildContext? caller;

  @override
  Object? invoke(_Run intent, [BuildContext? context]) {
    caller = context;
    return null;
  }
}

void main() {
  testWidgets(
    'shortcut context actions receive the context of primary focus',
    (tester) async {
      final action = _RecordContext();
      final focus = FocusNode();
      addTearDown(focus.dispose);
      await tester.pumpWidget(
        Actions(
          actions: <Type, Action<Intent>>{_Run: action},
          child: Shortcuts(
            shortcuts: const <ShortcutActivator, Intent>{
              SingleActivator(LogicalKeyboardKey.enter): _Run(),
            },
            child: Focus(focusNode: focus, autofocus: true, child: const SizedBox()),
          ),
        ),
      );
      await tester.pump();
      expect(focus.hasPrimaryFocus, isTrue);
      expect(action.caller, isNull);
      await tester.sendKeyEvent(LogicalKeyboardKey.enter);
      expect(action.caller, same(focus.context));
    },
  );
}
