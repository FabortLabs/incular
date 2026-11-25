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
dispatch apply to subsequent events. A panic does not prevent later
handlers or pressed-state updates. That case (:529) stays partial: Rust
collects typed payloads and logs the event context rather than producing
FlutterErrorDetails. Fourteen focused Rust checks passed.

## Irregular events and keyboard diagnostics

The exact pinned originals :573 and :593 passed. Rust tests confirm
the three opt-in event/state diagnostics, then compare error markers and
pressed sets through all seven irregular-key steps. Duplicate downs and
repeats can update the logical key, unmatched ups clear nothing, and
mismatched ups clear the physical position. All events still reach
handlers. Output is per-runtime and disabled by default; the Rust debug
representation of keys need not match Dart's string format. Sixteen
focused Rust tests passed.
