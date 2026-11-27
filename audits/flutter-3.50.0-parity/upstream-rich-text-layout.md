# RichText and inline child layout

Pinned Flutter: `3.50.0` (`ab2912035e160d9cad6306df307576da6128ebdb`).

- `widgets/rich_text_test.dart:169` mounts an 800×600 surface with an
  `IntrinsicHeight` paragraph and a full-width inline row between “Start”
  and “End.” Flutter and Incular use 16-pixel text, a 16-pixel inline box,
  and produce a 48-pixel intrinsic height. The native regression also
  checks the individual line heights and the inline box at y=16.
- `widgets/rich_text_test.dart:313` mounts a rich paragraph on a zero-size
  surface. The native test checks the actual render bounds and runs the
  retained compositor at zero size without an error.

The paragraph previously used baseline alignment by default instead of
Flutter's bottom alignment. After correcting the default, the synthetic
zero-width shaping character still added one pixel to the otherwise
inline-only line. The engine now measures such a line using its actual
bottom-aligned boxes before applying strut and snapping line geometry.
No renderer change is involved. The authored pinned
`widget_span_line_metrics_test.dart` also isolates the line height at
several box heights. This is not a claim of exact per-font baseline or
mixed-script inline coverage.

```sh
INCULAR_FLUTTER_AHEM_PATH="$PWD/.tmp/flutter/packages/flutter_tools/static/Ahem.ttf" \
  cargo test -p incular-widgets --test flutter350_rich_text_layout
```
