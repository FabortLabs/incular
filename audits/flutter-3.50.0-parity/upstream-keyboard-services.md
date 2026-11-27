# Keyboard services

Flutter reference: pinned Flutter 3.50.0 checkout.

## Control-key label classification

The exact `services/keyboard_key_test.dart:50` registration passed with
the same six control labels and four non-control labels. Two focused
Rust tests also check C0/C1 endpoints, Unicode joiners and emoji, and
multi-character labels. The predicate classifies one label rather than
changing native keyboard dispatch or text filtering.

## Pressed keys and lock state

The exact `services/hardware_keyboard_test.dart:18,105` registrations
passed with the pinned KeyData variant. Incular runtime tests send all
seven corresponding down, repeat and up events and check the physical
and portable logical sets, lock toggles and membership queries. The
portable code distinguishes the keypad position while reporting its
logical text as "1", rather than importing Dart's key ID scheme.
A control verifies that a changed release label still clears its
physical position and that a repeated lock key does not toggle again.
Eleven focused Rust tests passed. No native lock querying, full key-ID
table, disconnected keyboard or synthesized lifecycle sequences were
verified by these cases.

## Window-wide handler order and failure isolation

Both pinned variants of `services/hardware_keyboard_test.dart:164` and
`:529` passed. Rust pairs the nine dispatch steps of :164, including
duplicate registrations, removal, all-callback delivery even when one
handler accepts, and focused notification after global handlers. A
snapshot control verifies that edits to the registration list during
dispatch apply to subsequent events.

The :529 keyboard error reporter receives the original typed panic
payload and key event in both raw-only and typed-plus-raw transit.
A later handler still runs, the failed key-down remains pressed, and
key-up clears it without producing another report after the faulty
handler is removed. The event is structured data, not a reconstructed
log line. The default with no installed reporter still logs the
failure. Dart's diagnostic presentation is not copied.

## Irregular events and keyboard diagnostics

The exact pinned originals :573 and :593 passed. Rust tests confirm
the three opt-in event/state diagnostics, then compare error markers and
pressed sets through all seven irregular-key steps. Duplicate downs and
repeats can update the logical key, unmatched ups clear nothing, and
mismatched ups clear the physical position. All events still reach
handlers. Output is per-runtime and disabled by default; the Rust debug
representation of keys need not match Dart's string format. Sixteen
focused Rust tests passed.

## Key-message handler failure isolation

The exact pinned `services/hardware_keyboard_test.dart:481` registration
passed. The runtime records the key-down before the optional key-message
callback. Its failure reporter receives the original integer panic payload
and the combined typed/raw message for A, not just the portable hardware
event. The input remains pressed. Disabling only the message route and
sending key-up clears pressed state without another report.

The window-wide hardware handler still runs first and combines its result
with the custom message callback without short-circuiting. Setting the
message callback replaces focus routing, clearing it disables focus routing,
and restoring the default reinstates it. The separate authored control
checks that lifecycle; the Rust control also checks typed-only, raw-only,
and mirrored portable messages. This does not claim parity for GTK, the
native Flutter transit queue, synthesized repeats, or abort-on-panic builds.
