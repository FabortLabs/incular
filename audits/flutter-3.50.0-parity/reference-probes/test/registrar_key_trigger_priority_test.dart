import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

class _Keyed extends Intent {
  const _Keyed();
}

class _Character extends Intent {
  const _Character();
}

void main() {
  for (final characterFirst in [false, true]) {
    testWidgets('logical trigger wins with characterFirst=$characterFirst', (
      tester,
    ) async {
      final calls = <String>[];
      final key = GlobalKey();
      await tester.pumpWidget(
        ShortcutRegistrar(
          child: Actions(
            actions: {
              _Keyed: CallbackAction<_Keyed>(
                onInvoke: (_) => calls.add('single'),
              ),
              _Character: CallbackAction<_Character>(
                onInvoke: (_) => calls.add('character'),
              ),
            },
            child: Focus(autofocus: true, child: SizedBox(key: key)),
          ),
        ),
      );
      final registry = ShortcutRegistry.of(key.currentContext!);
      ShortcutRegistryEntry addKey() => registry.addAll(const {
        SingleActivator(LogicalKeyboardKey.keyC): _Keyed(),
      });
      ShortcutRegistryEntry addCharacter() =>
          registry.addAll(const {CharacterActivator('c'): _Character()});
      final first = characterFirst ? addCharacter() : addKey();
      final second = characterFirst ? addKey() : addCharacter();
      await tester.pumpAndSettle();
      await tester.sendKeyEvent(LogicalKeyboardKey.keyC, character: 'c');
      expect(calls, ['single']);

      (characterFirst ? second : first).dispose();
      expect(registry.shortcuts.keys.single, isA<CharacterActivator>());
      // The registrar publishes entry removals in a post-frame callback.
      tester.binding.scheduleFrame();
      await tester.pump();
      await tester.pumpAndSettle();
      await tester.sendKeyEvent(LogicalKeyboardKey.keyC, character: 'c');
      expect(calls, ['single', 'character']);
      (characterFirst ? first : second).dispose();
    });
  }
}
