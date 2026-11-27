# Synchronous diagnostic timeline

All nine pinned registrations in `foundation/timeline_test.dart`
passed. Native capture defaults to disabled, rejects collection
while disabled, drains recorded blocks on collection, and forgets
completed timings on reset. A missing aggregate returns the requested
label with zero count and duration. A synchronous callable returns
its actual result and records an interval on the same monotonic
clock. Instant events reach a separate sink and do not become timed
blocks.

The recorder preserves all 5,000 timings across fixed 500-entry
slices and the exact order of 1,750 distinct names. Aggregation
preserves the first appearance of each name and is deferred until
requested. An authored pinned test also checks nested completion
order, reset inside an open block, and capture of a throwing body
without replacing the exception. The corresponding native
tests check those cases and the lifetime of a scope across disabled
and re-enabled capture.

Incular uses an explicitly owned collector rather than a global
Dart VM tracing session. Enabling it is deliberate, including in an
optimized build; the inactive capture does not allocate buffers.
No native tracing transport or real-time benchmark is claimed.
All 22 focused timeline and diagnostic controls passed.
