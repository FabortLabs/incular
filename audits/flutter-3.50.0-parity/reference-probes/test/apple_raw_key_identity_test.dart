import 'dart:typed_data';

import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test(
    'Apple layouts keep their distinct Unicode identity and source text',
    () {
      RawKeyDownEvent decode(
        String keymap,
        int code,
        String? source,
        String label,
      ) {
        return RawKeyEvent.fromMessage({
          'type': 'keydown',
          'keymap': keymap,
          'keyCode': code,
          if (source != null) 'characters': source,
          'charactersIgnoringModifiers': label,
          'modifiers': 0,
        }) as RawKeyDownEvent;
      }

      final ios = decode('ios', 4, '😊', '😊');
      expect(ios.logicalKey.keyId, 0xd83dde0a);
      expect(ios.character, '😊');
      expect(
        decode('ios', 0x50, '', 'UIKeyInputEnd').logicalKey,
        LogicalKeyboardKey.enter,
      );
      expect(decode('ios', 0x29, '', '').character, isNull);
      expect(decode('macos', 0x35, '', '').character, '');
      expect(decode('macos', 0x35, null, '').character, isNull);
    },
  );

  testWidgets(
    'macOS Fn skips its physical release even after a logical override',
    (tester) async {
      final events = <RawKeyEvent>[];
      RawKeyboard.instance.addListener(events.add);
      addTearDown(() => RawKeyboard.instance.removeListener(events.add));
      Future<void> send(
        String type,
        int code,
        int modifiers, {
        int? logical,
      }) async {
        ByteData? reply;
        await tester.binding.defaultBinaryMessenger.handlePlatformMessage(
          SystemChannels.keyEvent.name,
          SystemChannels.keyEvent.codec.encodeMessage({
            'type': type,
            'keymap': 'macos',
            'keyCode': code,
            'modifiers': modifiers,
            if (logical != null) 'specifiedLogicalKey': logical,
          }),
          (data) => reply = data,
        );
        if (code == 0x3f) {
          expect(SystemChannels.keyEvent.codec.decodeMessage(reply), {
            'handled': true,
          });
        }
      }

      await send('keydown', 0x3f, 0x800000);
      expect(RawKeyboard.instance.keysPressed, isEmpty);
      await send('keydown', 0x6f, 0x800000);
      expect(RawKeyboard.instance.keysPressed, {LogicalKeyboardKey.f12});
      await send('keyup', 0x3f, 0, logical: LogicalKeyboardKey.keyA.keyId);
      expect(RawKeyboard.instance.keysPressed, {LogicalKeyboardKey.f12});
      await send('keyup', 0x6f, 0);
      expect(RawKeyboard.instance.keysPressed, isEmpty);
      expect(events.map((event) => event.logicalKey), [
        LogicalKeyboardKey.f12,
        LogicalKeyboardKey.f12,
      ]);
    },
  );
}
