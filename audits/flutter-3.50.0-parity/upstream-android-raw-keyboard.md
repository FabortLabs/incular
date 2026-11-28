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

## Mounted responses, held modifiers and soft keyboard

Pinned Android originals at raw keyboard :307, :591, :929 and
:1389 pair with the mounted runtime target. Generic-only modifier
bits reconstruct both sides; an inconsistent Caps Lock keydown
still reaches the raw observer and keeps its physical press.
Unfocused A replies unhandled; the same packet after attaching a
focused raw handler replies handled and reports repeat based on
the physical press instead of the independent repeat-count field.

The pinned hardware-keyboard :139 Android branch is now paired
with the same explicit Android input in Incular: an already-held
left Shift produces one reconstructed hardware down before the
supplied A. Its release is checked as an adjacent native case.
The prior Web branch replay also remains green.

An Android packet marked as virtual by flags or device ID still
edits the focused field without switching touch-mode highlights.
The existing normalized platform highlight registration is
unchanged; this adds coverage of the explicit legacy route.
