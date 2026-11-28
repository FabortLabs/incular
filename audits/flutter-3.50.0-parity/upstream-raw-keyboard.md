# Raw keyboard behavior

## Fuchsia key payload interpretation

Pinned `services/raw_keyboard_test.dart:1546,1574,1610,1624,1636,1664`
passed. The native explicit legacy payload adapter checks all 13
individual/side-specific masks and the 12 Caps Lock combinations.
Printable A carries its code point, key label and USB HID identity.
Escape and left Shift remain named nonprintable events without labels.
The original metadata survives construction; changing any part of the
packet can be observed through value equality.

The diagnostic registration at **1648** is partial: the same decimal
HID, code point and modifiers appear in Rust's Debug output, not with
Dart's type spelling and object hash suffix. The tests use the explicit
host payload adapter, not a physical Fuchsia machine or a claim about
every Flutter JSON route. All six focused native tests passed.

## Passive listener ordering and error recovery

Pinned **1018** passed in both legacy raw-only and paired UI/raw transit
modes. A mounted listener sees each raw event first, followed by the
window-level subscriptions in registration order. Adding the same
callback twice gives two observations, and dropping the first
subscription preserves the second in its original place. No raw
listener consumes the event; typed-only events never invent a raw
counterpart.

Pinned **1080** passed. Incular previously let a mounted raw
observer's panic escape dispatch. Window and mounted listeners now
report the original event and panic payload while other listeners
continue. Hardware state is recorded before callbacks, so KeyA is
pressed after the failed down and cleared normally on up.

## Pressed-key reconciliation from raw modifier flags

Pinned originals `services/raw_keyboard_test.dart:338` (Fuchsia,
`RawKeyEvent` variant) and `:520` (Web) passed. The native runtime
replays the corresponding platform packets in
`flutter350_raw_pressed_modifiers`: an A press can reveal a Shift
that was already held; a subsequent packet without Shift clears it.
Fuchsia specifies the exact side(s). A Web mask with no known side
defaults to left but preserves a right side already present.
Release uses the physical identity even if the logical layout
changes. The reconciled state is visible separately from modern
hardware state. Paired messages retain their independently supplied
modern events (see the raw-only projection below).

Adjacent native controls cover right Alt reported as AltGraph and
CapsLock remapped to a regular logical key. The original
browser-only registrations for those scenarios are not counted here.
This check replays framework packets rather than using Fuchsia or
browser hardware.

## Raw-only modifier projection into hardware callbacks

The pinned `hardware_keyboard_test.dart:139` passes on Linux
using its Android packet branch. The independent
`raw_keyboard_manager_modifiers_test.dart` probe exercises
both raw-only and typed-then-raw Web packets on the pinned
framework. The native tests use the same Web fields and
distinguish the origin of each modern callback. Only raw-only
input emits ShiftLeft before A and the corresponding ShiftLeft
release before A's release. Passive raw listeners get only the
actual A packet. Paired input uses the supplied typed A and
does not infer another hardware key from its legacy flags.

The original registration is partial, not counted as paired:
the exact Android branch ran only on the reference, and the Web
branch is verified through an authored packet replay, not via a
browser test runner. The reconstructed events in this step reach
global hardware handlers; focus batch delivery is tracked
separately.
