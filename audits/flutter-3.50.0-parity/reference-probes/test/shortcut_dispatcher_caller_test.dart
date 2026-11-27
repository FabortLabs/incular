import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

class _Run extends Intent {
  const _Run();
}

class _RunAction extends Action<_Run> {
  _RunAction(this.calls);
  final List<String> calls;

  @override
  bool consumesKey(_Run intent) => false;

  @override
  Object? invoke(_Run intent) {
    calls.add('action');
    return null;
  }
}

class _Dispatcher extends ActionDispatcher {
  _Dispatcher(this.name, this.calls);
  final String name;
  final List<String> calls;

  @override
  (bool, Object?) invokeActionIfEnabled(
    Action<Intent> action,
    Intent intent, [
    BuildContext? context,
  ]) {
    calls.add(name);
    return super.invokeActionIfEnabled(action, intent, context);
  }

  @override
  Object? invokeAction(
    Action<Intent> action,
    Intent intent, [
    BuildContext? context,
  ]) {
    calls.add(name);
    return super.invokeAction(action, intent, context);
  }
}

void main() {
  testWidgets(
    'shortcut dispatch uses focused context and stops framework fallback',
    (tester) async {
      final calls = <String>[];
      final key = GlobalKey();
      await tester.pumpWidget(
        Focus(
          onKeyEvent: (node, event) {
            if (event is KeyDownEvent) {
              calls.add('fallback');
            }
            return KeyEventResult.ignored;
          },
          child: Actions(
            dispatcher: _Dispatcher('owner dispatcher', calls),
            actions: {_Run: _RunAction(calls)},
            child: Shortcuts(
              shortcuts: const {
                SingleActivator(LogicalKeyboardKey.enter): _Run(),
              },
              child: Actions(
                dispatcher: _Dispatcher('caller dispatcher', calls),
                actions: const {},
                child: Focus(autofocus: true, child: SizedBox(key: key)),
              ),
            ),
          ),
        ),
      );

      Actions.invoke(key.currentContext!, const _Run());
      expect(calls, ['owner dispatcher', 'action']);
      calls.clear();
      await tester.sendKeyDownEvent(LogicalKeyboardKey.enter);
      expect(calls, ['caller dispatcher', 'action']);
      await tester.sendKeyUpEvent(LogicalKeyboardKey.enter);
    },
  );
}
