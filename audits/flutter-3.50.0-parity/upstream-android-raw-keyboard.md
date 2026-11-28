# Android legacy keyboard packets

## Packet identity, modifiers and character fields

The pinned `RawKeyEventDataAndroid` group in
`services/raw_keyboard_test.dart` passed. Eleven originals pair with
the native packet tests: independent and combined flags,
printable/control/modifier identities, keyboard and joystick
arrows, gamepad-center scan identity, device ID, repeat count,
and value equality. An explicit capital event character is not
the unmodified logical key label. Joystick arrow fallback is
used only when the scan-code catalog cannot identify the key.

Flutter's value equality uses six fields. Additional device,
source and repeat fields remain available without changing
equality. The Dart diagnostic formatting registration remains
partial, not counted. The mounted handled-response case is
separate from these data tests. Added native controls cover
combining code-point markers and numpad precedence.

The numeric tables are reproducible from pinned Flutter with
`extract-android-raw-key-catalog.py`; the Windows generator
continues to reproduce its prior output byte for byte. These
are explicit framework packets, not Android-device execution.
