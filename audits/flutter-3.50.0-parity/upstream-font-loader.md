# Ordered application font loading

Both pinned `services/font_loader_test.dart:22,39` originals passed.
The native fixture receives the exact registered arrays under the
same family and leaves the ready second face pending while the
first asset is delayed. It tests the real ordered loading loop
with an injected registrar, like Flutter's test subclass.

The authored pinned probe also checks a three-byte subview of a
five-byte asset, a pending registration before the next face, and
rejection of additions and loads once loading starts. Native
controls match these and verify that a failed asset stops later
registration without undoing earlier registrations. They also
load a real Inter face into an existing text engine: it replaces
the cached fallback, and the actual shaped run uses the registered
font bytes.

One-shot state and failures become typed Rust errors. Font futures
are polled in insertion order; parallel fetching can be driven
by the caller, while registration remains ordered. Thirteen focused
native checks passed. One preexisting opt-in font variation fixture
was not run.
