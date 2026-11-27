import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

class _Run extends Intent {
  const _Run();
}

class _Gated extends ContextAction<_Run> {
  _Gated(this.anchor);
  final GlobalKey anchor;
  bool enabled = true;
  int invocations = 0;

  @override
  bool isEnabled(_Run intent, [BuildContext? context]) {
    return enabled && identical(context, anchor.currentContext);
  }

  @override
  Object? invoke(_Run intent, [BuildContext? context]) {
    expect(context, same(anchor.currentContext));
    invocations += 1;
    return null;
  }
}

void main() {
  testWidgets(
    'a saved action handler rechecks enablement with the original caller',
    (tester) async {
      final anchor = GlobalKey();
      final action = _Gated(anchor);
      await tester.pumpWidget(
        Actions(
          actions: <Type, Action<Intent>>{_Run: action},
          child: SizedBox(key: anchor),
        ),
      );
      final caller = anchor.currentContext!;
      final handler = Actions.handler(caller, const _Run())!;
      handler();
      expect(action.invocations, 1);

      action.enabled = false;
      expect(Actions.handler(caller, const _Run()), isNull);
      expect(Actions.maybeInvoke(caller, const _Run()), isNull);
      handler();
      expect(action.invocations, 1);

      action.enabled = true;
      handler();
      expect(action.invocations, 2);
    },
  );
}
