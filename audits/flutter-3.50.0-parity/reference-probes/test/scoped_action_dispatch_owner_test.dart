import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

class _Run extends Intent {
  const _Run();
}

class _RunAction extends Action<_Run> {
  _RunAction(this.onInvoke, {required this.enabled});
  final bool enabled;
  final VoidCallback onInvoke;

  @override
  bool isEnabled(_Run intent) => enabled;

  @override
  Object? invoke(_Run intent) {
    onInvoke();
    return true;
  }
}

class _Dispatcher extends ActionDispatcher {
  _Dispatcher(this.calls);
  final List<String> calls;

  @override
  Object? invokeAction(
    Action<Intent> action,
    Intent intent, [
    BuildContext? context,
  ]) {
    calls.add('outer dispatcher');
    return super.invokeAction(action, intent, context);
  }
}

void main() {
  testWidgets(
    'a matching inner action uses the outer dispatcher if it has none',
    (tester) async {
      final calls = <String>[];
      final key = GlobalKey();
      Widget build(bool enabled) => Actions(
        dispatcher: _Dispatcher(calls),
        actions: {
          _Run: _RunAction(() => calls.add('outer action'), enabled: true),
        },
        child: Actions(
          actions: {
            _Run: _RunAction(() => calls.add('inner action'), enabled: enabled),
          },
          child: SizedBox(key: key),
        ),
      );

      await tester.pumpWidget(build(true));
      expect(Actions.maybeInvoke(key.currentContext!, const _Run()), true);
      expect(calls, ['outer dispatcher', 'inner action']);

      calls.clear();
      await tester.pumpWidget(build(false));
      expect(Actions.maybeInvoke(key.currentContext!, const _Run()), isNull);
      expect(calls, isEmpty);
    },
  );
}
