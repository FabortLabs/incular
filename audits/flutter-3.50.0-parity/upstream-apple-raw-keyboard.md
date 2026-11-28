# Legacy macOS and iOS keyboard packets

Flutter 3.50.0 reference: `packages/flutter/test/services/raw_keyboard_test.dart`.
The typed packet interface is optional and does not change either host's
native OS event transport.

## Key identity, label, and modifiers

`incular-gestures/tests/flutter350_apple_raw_keyboard.rs` checks both
platforms' individual and combined modifier masks. Side bits without
their aggregate bit describe the side but do not report an active
modifier. Aggregate-only bits apply to both sides; CapsLock combines
with a sided modifier. Fn and keypad flag bits are not independently
pressable modifiers.

AppKit uses the native position, lowers the eligible single-rune key
identity without replacing the source text, and declines multi-rune
labels as key identities. An embedder layout override precedes that
derivation. UIKit uses USB-derived positions, recognizes special
labels before the ordinary key-code table, and packs two UTF-16 units
for its fallback logical key. Neither turns Escape, Shift or the
arrow key into a printable identity. Explicit empty character text
remains empty on Mac but is absent on iOS. Packet equality
uses the reference value fields, excluding the Mac layout override.

## Runtime held keys

`incular-runtime/tests/flutter350_apple_raw_dispatch.rs` sends the
packets through the actual key ingress. It checks both platforms'
single-side and aggregate-only snapshots, including release. An
iOS CapsLock press with no mask still reaches the window listener
and is held until up. A Mac Fn down and matching physical release
are both suppressed, even if the release reports a changed logical
key; F12 remains independently observable.

The authored `apple_raw_key_identity_test.dart` checks exact iOS
astral identity, empty versus missing character data, and the
Mac Fn suppression sequence. Dart's diagnostics string formatting
and assertions for labels outside the typed domain are not credited.
