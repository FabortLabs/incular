# Flutter 3.50 allocation event dispatch

Exact `scheduler/ticker_test.dart:208` passed: a ticker produces one
create and one dispose event for the same identity, in that order.
Rust sends stable IDs rather than references, so listening does not
extend the lifetime of the observed object. The debug-enable default
also matches `foundation/memory_allocations_test.dart:172`; instrumentation
is disabled in Rust release builds.

The pinned foundation registrations :42, :62, :101, and :131 passed.
Rust reproduces listener add/remove/delivery, deferral of registrations
made during the current event, immediate skipping of a removed
registration, and continuing past two throwing observers in order.
These four remain partial: Dart also checks Image/Picture SDK hook
installation, and its error reporter aggregates structured exceptions.
Rust catches observer panics, the standard panic hook reports each,
and the dispatch call returns the number of failures. Full SDK and
other framework publishers are not credited.

## Notifier publishers

Pinned :176 passed. The two neutral notifier lifetimes now pair:
a ValueNotifier is observed immediately and ends when its final Rust
owner drops; a plain ChangeNotifier is observed on its first
listener and ends once on explicit disposal. Cloning the handle,
adding later listeners, or disposing a never-observed notifier
does not add events. The original stays partial because its
additional Dart UI Image/Picture publishers are not paired. The
separate :155 SDK hook unsubscription case ran on Flutter only.

## Render-object and layer publishers

Pinned `rendering/memory_allocations_test.dart:23` pairs a real render
object and a compositor layer, each with one creation and one disposal
for its own stable identity. The Rust fixture further verifies that a
retained render object recolored during an update produces neither an
extra creation nor a disposal. The instrumentation is debug-only.
