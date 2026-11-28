import 'dart:ui' as ui;

import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';

Map<String, Object> packet(String type, int mask) => <String, Object>{
  'type': type,
  'keymap': 'web',
  'code': 'KeyA',
  'key': 'a',
  'location': 0,
  'metaState': mask,
  'keyCode': 65,
};

void main() {
  testWidgets('raw-only Web packets fill missing hardware modifier events', (
    tester,
  ) async {
    final manager = tester.binding.keyEventManager;
    final events = <(PhysicalKeyboardKey, bool, bool)>[];
    final raw = <PhysicalKeyboardKey>[];
    bool hardware(KeyEvent event) {
      events.add((event.physicalKey, event is! KeyUpEvent, event.synthesized));
      return false;
    }

    void legacy(RawKeyEvent event) => raw.add(event.physicalKey);
    HardwareKeyboard.instance.addHandler(hardware);
    RawKeyboard.instance.addListener(legacy);
    addTearDown(() {
      HardwareKeyboard.instance.removeHandler(hardware);
      RawKeyboard.instance.removeListener(legacy);
    });

    await manager.handleRawKeyMessage(
      packet('keydown', RawKeyEventDataWeb.modifierShift),
    );
    expect(events, <(PhysicalKeyboardKey, bool, bool)>[
      (PhysicalKeyboardKey.shiftLeft, true, true),
      (PhysicalKeyboardKey.keyA, true, false),
    ]);
    expect(raw, [PhysicalKeyboardKey.keyA]);

    await manager.handleRawKeyMessage(packet('keyup', 0));
    expect(events.skip(2), <(PhysicalKeyboardKey, bool, bool)>[
      (PhysicalKeyboardKey.shiftLeft, false, true),
      (PhysicalKeyboardKey.keyA, false, false),
    ]);
    expect(raw, [PhysicalKeyboardKey.keyA, PhysicalKeyboardKey.keyA]);
  });

  testWidgets(
    'typed then raw leaves hardware keys to the supplied typed packet',
    (tester) async {
      final manager = tester.binding.keyEventManager;
      final events = <PhysicalKeyboardKey>[];
      bool hardware(KeyEvent event) {
        expect(event.synthesized, false);
        events.add(event.physicalKey);
        return false;
      }

      HardwareKeyboard.instance.addHandler(hardware);
      addTearDown(() => HardwareKeyboard.instance.removeHandler(hardware));

      manager.handleKeyData(
        ui.KeyData(
          timeStamp: Duration.zero,
          type: ui.KeyEventType.down,
          physical: PhysicalKeyboardKey.keyA.usbHidUsage,
          logical: LogicalKeyboardKey.keyA.keyId,
          character: 'a',
          synthesized: false,
        ),
      );
      await manager.handleRawKeyMessage(
        packet('keydown', RawKeyEventDataWeb.modifierShift),
      );

      expect(events, [PhysicalKeyboardKey.keyA]);
      expect(HardwareKeyboard.instance.physicalKeysPressed, {
        PhysicalKeyboardKey.keyA,
      });
      expect(RawKeyboard.instance.physicalKeysPressed, {
        PhysicalKeyboardKey.keyA,
        PhysicalKeyboardKey.shiftLeft,
      });
    },
  );
}
