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
