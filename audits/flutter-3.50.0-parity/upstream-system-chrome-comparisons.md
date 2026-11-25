# System chrome

Flutter reference: pinned Flutter 3.50.0 checkout.

## System UI overlay style values

Five exact `services/system_chrome_test.dart` registrations (:235, :250,
:289, :312, :331) pair eight unset fields, populated diagnostics and their
ordering, equality/hash relations, and copy overrides. Tests compare sRGB
components and diagnostic field labels, not the numeric debug identifier
or exact hashes across runtimes. Rust also checks explicit false overrides.
Eight focused Rust tests and five Flutter originals passed. This value slice
does not verify native bar drawing or non-sRGB colors.

## Detach resets the delivery cache

The exact `services/system_chrome_test.dart:363` case pairs the sequence
at the host boundary: one initial delivery, no duplicate while attached,
and a new delivery of the same style after detachment and a new request.
The mobile adapter invalidates the delivered-value cache on surface loss
and replacement. Another Rust control verifies that a queued request
survives invalidation. Five focused runtime tests, five headless mobile
engine tests, and the exact Flutter original passed. No physical device
bar rendering was tested.
