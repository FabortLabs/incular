import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets(
    'one raw packet batches typed corrections through the focus chain',
    (tester) async {
      for (final handleModifier in [false, true]) {
        tester.binding.keyEventManager.clearState();
        HardwareKeyboard.instance.clearState();
        RawKeyboard.instance.clearKeysPressed();
        final log = <String>[];
        await tester.pumpWidget(
          Focus(
            onKeyEvent: (node, event) {
              log.add(event.synthesized ? 'outer shift' : 'outer A');
              return KeyEventResult.ignored;
            },
            onKey: (node, event) {
              log.add('outer raw');
              return KeyEventResult.ignored;
            },
            child: Focus(
              autofocus: true,
              onKeyEvent: (node, event) {
                log.add(event.synthesized ? 'inner shift' : 'inner A');
                return handleModifier && event.synthesized
                    ? KeyEventResult.handled
                    : KeyEventResult.ignored;
              },
              onKey: (node, event) {
                log.add('inner raw');
                return KeyEventResult.ignored;
              },
              child: const SizedBox.shrink(),
            ),
          ),
        );
        await tester.pumpAndSettle();
        final result = await tester.binding.keyEventManager.handleRawKeyMessage(
          <String, Object>{
            'type': 'keydown',
            'keymap': 'web',
            'code': 'KeyA',
            'key': 'a',
            'location': 0,
            'metaState': RawKeyEventDataWeb.modifierShift,
            'keyCode': 65,
          },
        );
        expect(log, [
          'inner shift',
          'inner A',
          'inner raw',
          if (!handleModifier) ...['outer shift', 'outer A', 'outer raw'],
        ]);
        expect(result['handled'], handleModifier);
        await tester.pumpWidget(const SizedBox.shrink());
      }
    },
  );
}
