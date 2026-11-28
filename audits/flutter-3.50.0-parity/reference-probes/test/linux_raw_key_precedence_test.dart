import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('Linux toolkit fallbacks and numpad precede layout character', () {
    for (final (toolkit, keypad, plane) in [
      ('gtk', 0xffb1, LogicalKeyboardKey.gtkPlane),
      ('glfw', 321, LogicalKeyboardKey.glfwPlane),
    ]) {
      RawKeyDownEvent decode(int scan, int key, int unicode) =>
          RawKeyEvent.fromMessage({
            'type': 'keydown',
            'keymap': 'linux',
            'toolkit': toolkit,
            'scanCode': scan,
            'keyCode': key,
            'unicodeScalarValues': unicode,
            'modifiers': 0,
          }) as RawKeyDownEvent;
      final numeric = decode(87, keypad, 49);
      expect(numeric.physicalKey, PhysicalKeyboardKey.numpad1);
      expect(numeric.logicalKey, LogicalKeyboardKey.numpad1);
      expect(numeric.character, '1');
      final unknown = decode(0xffff, 0x10000000, 0);
      expect(
        unknown.physicalKey.usbHidUsage,
        LogicalKeyboardKey.webPlane + 0xffff,
      );
      expect(unknown.logicalKey.keyId, plane | 0x10000000);
    }
  });
}
