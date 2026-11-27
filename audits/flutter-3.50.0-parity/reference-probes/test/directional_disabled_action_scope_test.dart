import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

class _Run extends Intent {
  const _Run();
}

void main() {
  testWidgets('disabled directional detector omits its action and shortcut maps', (tester) async {
    final anchor = GlobalKey();
    final focus = FocusNode();
    addTearDown(focus.dispose);
    var local = 0;
    var parent = 0;

    Future<void> mount(bool enabled) async {
      await tester.pumpWidget(
        MediaQuery(
          data: const MediaQueryData(navigationMode: NavigationMode.directional),
          child: Actions(
            actions: <Type, Action<Intent>>{
              _Run: CallbackAction<_Run>(onInvoke: (_) => ++parent),
            },
            child: FocusableActionDetector(
              enabled: enabled,
              focusNode: focus,
              actions: <Type, Action<Intent>>{
                _Run: CallbackAction<_Run>(onInvoke: (_) => ++local),
              },
              shortcuts: const <ShortcutActivator, Intent>{
                SingleActivator(LogicalKeyboardKey.enter): _Run(),
              },
              child: SizedBox(key: anchor),
            ),
          ),
        ),
      );
      focus.requestFocus();
      await tester.pump();
      expect(focus.hasPrimaryFocus, isTrue);
    }

    await mount(true);
    await tester.sendKeyEvent(LogicalKeyboardKey.enter);
    expect((local, parent), (1, 0));

    await mount(false);
    await tester.sendKeyEvent(LogicalKeyboardKey.enter);
    expect((local, parent), (1, 0));
    Actions.invoke(anchor.currentContext!, const _Run());
    expect((local, parent), (1, 1));
  });
}
