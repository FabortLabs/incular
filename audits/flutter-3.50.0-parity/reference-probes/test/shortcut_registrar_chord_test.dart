import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('registered key set precedes a character fallback only when exact', (tester) async {
    late ShortcutRegistry registry;
    int chords = 0;
    int characters = 0;
    const a = LogicalKeyboardKey.keyA;
    const b = LogicalKeyboardKey.keyB;
    const c = LogicalKeyboardKey.keyC;

    await tester.pumpWidget(
      ShortcutRegistrar(
        child: Builder(
          builder: (context) {
            registry = ShortcutRegistry.of(context);
            return Actions(
              actions: {VoidCallbackIntent: VoidCallbackAction()},
              child: const Focus(autofocus: true, child: SizedBox()),
            );
          },
        ),
      ),
    );
    final binding = registry.addAll({
      LogicalKeySet(a, b): VoidCallbackIntent(() => chords += 1),
    });
    final fallback = registry.addAll({
      const CharacterActivator('b'): VoidCallbackIntent(() => characters += 1),
    });
    await tester.pump();

    expect(await tester.sendKeyDownEvent(a), isFalse);
    expect(await tester.sendKeyDownEvent(b), isTrue);
    expect((chords, characters), (1, 0));
    expect(await tester.sendKeyUpEvent(b), isFalse);
    expect((chords, characters), (1, 0));

    expect(await tester.sendKeyDownEvent(c), isFalse);
    expect(await tester.sendKeyDownEvent(b), isTrue);
    expect((chords, characters), (1, 1));
    expect(await tester.sendKeyUpEvent(c), isFalse);
    expect((chords, characters), (1, 1));

    expect(await tester.sendKeyRepeatEvent(b), isTrue);
    expect((chords, characters), (2, 1));
    binding.dispose();
    tester.binding.scheduleFrame();
    await tester.pump();
    await tester.pumpAndSettle();
    expect(await tester.sendKeyRepeatEvent(b), isTrue);
    expect((chords, characters), (2, 2));
    expect(await tester.sendKeyUpEvent(b), isFalse);
    expect(await tester.sendKeyUpEvent(a), isFalse);
    expect((chords, characters), (2, 2));
    expect(HardwareKeyboard.instance.logicalKeysPressed, isEmpty);
    fallback.dispose();
    await tester.pump();
  });
}
