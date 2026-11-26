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

## Imperative delivery and remaining platform commands

Pinned :14 verifies one deferred light-style delivery after dark,
complete nullable payload, no duplicate, and a two-field contrast
override. The corresponding Rust test checks all eight typed fields
and the drain order. Runtime now requests a frame for a new style
on an otherwise idle root, which it previously did not. Repeating an
already delivered style leaves the root idle, and replacing a pending
change with the delivered style does not issue a redundant draw command.
This is partial because Flutter uses a microtask and a serialized
method call; Incular's host drains a typed value in its update.

The eight other selected originals (:88, :111, :136, :154, :174,
:189, :215, :393) passed only on Flutter. The orientation lock,
application-switcher metadata, overlay mode/manual overlay list,
visibility callback registration, and returned platform-error callback
have no corresponding Runtime service in this checkout. A fullscreen
window is not a substitute for a platform overlay policy, and successful
typed delivery does not verify the asynchronous error callback.

Ten focused Rust runtime controls passed. No native bar pixels, Android
phone, or iOS view controller were exercised.

## System chrome JSON commands

Seven previously open originals now pair: `:88, :111, :136,
:154, :174, :189, :215`. The service sends one JSON command to
flutter/platform with the ordered orientation enum strings,
nullable task-switcher metadata and unsigned ARGB, or the
specific mode/overlay arguments. Missing optional-platform plugins
complete successfully. A manual mode with an absent overlay list
is rejected before transport; an empty but supplied list is legal.
Clearing the visibility callback sends nothing; installing one
sends the one listener-registration command.

Native controls also decode real inbound visibility messages,
check both booleans and a malformed payload, and make sure a
removed callback no longer sees messages. All 19 selected native
checks passed. These are host messages, not proof of OS-level
orientation or bar drawing. Deferred overlay style and its
error-reporting original `:393` are separate from this command set.
