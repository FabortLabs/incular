# Pointer wheel signal resolution

Pinned Flutter 3.50.0 `ab2912035e`. All five exact originals in
`gestures/pointer_signal_resolver_test.dart` passed.

- Lines 40 and 45 pair: with no claimant the signal changes nothing,
  and the host receives a single response allowing its default action.
  A passive listener may still observe the wheel before that response.
- Lines 61 and 70 pair: the innermost (first) registration sees the
  same sample it registered, once. A second independent event resolves
  normally without replaying a prior registration. A claimed signal
  does not call the unclaimed-response callback.
- Line 86 is partial: both transformed callbacks can register for the
  same routed signal, but only the first receives its own local
  coordinates. Dart's transformed-object reference identity does not
  have a Rust value-type counterpart.

Thirty-nine focused Rust tests passed across retained wheel resolution,
scrolling, runtime input and allocation controls. The new host API
reports the default decision; no assertion is made about a browser or
other native adapter actually acting on it.

## Standalone transformed signal identity

Pinned `:86` now pairs at the direct resolver boundary. Each
transformed view has its own identity and local position while
retaining the exact same original signal. Both views register, but
only the first view's callback receives that very instance when the
original resolves. Equal raw contents on different originals do not
make them the same dispatch. Unclaimed default responses and resolver
reuse after an error are covered separately.

Three direct native tests and nine retained wheel controls passed.
The mounted wheel path retains its lightweight event values; it does
not pay for the optional shared handles.

## Nested retained pointer signal dispatch

An authored Rust regression reproduced an outer listener losing its
wheel claim when it synchronously dispatched a second wheel event to
another tree. The outer wheel was incorrectly allowed to reach the
platform default. Each dispatch now owns a scoped resolution, so
inner and outer winners receive only their own deltas. A winner can
also run a standalone handler without claiming an outer event.
An unwinding nested listener restores the correctly pending outer
session. Two nested and nine ordinary wheel tests passed. The common
single-dispatch storage remains inline. No pinned registration was added.
