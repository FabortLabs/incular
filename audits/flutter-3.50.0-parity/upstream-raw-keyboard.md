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
