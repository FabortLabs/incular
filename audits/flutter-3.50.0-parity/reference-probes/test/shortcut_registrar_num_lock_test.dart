import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets(
    'registered SingleActivator observes the hardware Num Lock mode',
    (tester) async {
      late ShortcutRegistry registry;
      var hits = 0;
      await tester.pumpWidget(
        ShortcutRegistrar(
          child: Builder(
            builder: (context) {
              registry = ShortcutRegistry.of(context);
              return Actions(
                actions: <Type, Action<Intent>>{
                  VoidCallbackIntent: VoidCallbackAction(),
                },
                child: const Focus(autofocus: true, child: SizedBox()),
              );
            },
          ),
        ),
      );
      final entry = registry.addAll(<ShortcutActivator, Intent>{
        const SingleActivator(
          LogicalKeyboardKey.numpad4,
          numLock: LockState.locked,
        ): VoidCallbackIntent(
          () => hits += 1,
        ),
      });
      await tester.pump();

      for (final (step, locked) in [false, true, false].indexed) {
        expect(
          HardwareKeyboard.instance.lockModesEnabled.contains(
            KeyboardLockMode.numLock,
          ),
          locked,
        );
        final before = hits;
        await tester.sendKeyDownEvent(LogicalKeyboardKey.numpad4);
        expect(hits, before + (locked ? 1 : 0));
        await tester.sendKeyUpEvent(LogicalKeyboardKey.numpad4);
        expect(hits, before + (locked ? 1 : 0));
        if (step < 2) {
          await tester.sendKeyEvent(LogicalKeyboardKey.numLock);
        }
      }
      expect(hits, 1);
      expect(HardwareKeyboard.instance.logicalKeysPressed, isEmpty);
      entry.dispose();
      await tester.pump();
    },
  );
}
