# Legacy Linux keyboard packets

Reference: Flutter 3.50.0 pin, `test/services/raw_keyboard_test.dart`.
This explicit compatibility ingress does not change the normal Linux host's
portable input path and does not add a toolkit dependency.

## Toolkit identities and modifier masks

`incular-gestures/tests/flutter350_linux_raw_keyboard.rs` verifies both
toolkits' individual and combined modifier tests with the exact masks and
key codes. The input's mask is the state **before** its own modifier
transition; a release clears the relevant flag. Both sides report the same
state, and ScrollLock, Fn and Symbol do not report flags. The position comes
from XKB independently of toolkit, Escape and Shift use their toolkit
key codes, and printing Q on the A position does not change the position.
U+10FFFF keeps both its valid identity and label. The GTK layout override
wins for logical identity, but does not replace the packet character.

The authored `linux_raw_key_precedence_test.dart` additionally exercises the
keypad-one identity ahead of a printable label and exact unknown physical
(Web) and logical (GTK or GLFW) planes. It does not add upstream count.

## Runtime modifier state

`incular-runtime/tests/flutter350_linux_raw_dispatch.rs` dispatches the
packets through the real runtime. The GLFW packet's shift flag fills both
sides and its release clears both without also clearing A. GTK's
group-switch sequence preserves a physical right Alt down with the
standard AltRight logical identity even when the mask omits Alt. A physical
CapsLock remapped to Backspace remains Backspace across a later packet and
clears on release.

The Rust packet is typed rather than a Dart message map. Dart-specific
out-of-width integer assertions, diagnostic rendering and equality
semantics (Dart does not compare `specifiedLogicalKey`) are not credited.
