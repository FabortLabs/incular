# Logical and physical view bounds

Two originals from Flutter `ab2912035e160d9cad6306df307576da6128ebdb` ran and passed:

- `rendering/view_test.dart:71`: compares view configurations and their hashes, varying the logical bounds, physical bounds, and device pixel ratio. The native `flutter350_view_configuration` fixture verifies these fields participate in equality and that equal configurations hash equally. It also checks the default, scale matrix, and whether a new ratio requires a matrix update.
- `rendering/view_constraints_test.dart:11`: a view using ratio `1.912500023841858` must present exactly `1008 × 2198` physical pixels. The native `flutter350_view_constraint_layout` fixture mounts and measures the actual retained root under the derived logical bounds, then verifies that converting the measured size yields the same physical dimensions. This covers the host size contract, not a window screenshot.

Incular keeps logical layout in f32, so it divides at host precision before narrowing. Conversion back to physical size scales before clamping to the host bounds. The raw host surface allocation is unchanged. The adjacent box-constraint arithmetic fixture also passed.

## Root paint bounds and debug overlay

Pinned `rendering/view_test.dart:28,82` passed. The runtime's root
paint bounds use the measured logical layout and device pixel ratio;
the native test exercises three real metrics changes, including 2.5×,
without feeding physical sizes back into widget layout.

Debug view callbacks draw on top of retained content, can remove a
later callback before its turn, and do not request a frame when added
or removed. The native test compares actual rectangle order before,
during and after registration, including previously retained buffers.
Incular keeps registrations view-local and returns a disposable
subscription rather than altering Flutter's static registry; release
builds do not install them. All four focused native tests passed.
