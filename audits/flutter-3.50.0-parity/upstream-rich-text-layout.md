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

## Ellipsized inline children

The authored pinned `ellipsized_inline_widget_presentation_test.dart`
mounts a rich paragraph without named spans and keeps the same green
inline child mounted across a width change. At 100 pixels, the one-line
ellipsis removes the placeholder's placement. Flutter does not paint the
green widget, send it a tap, or publish its annotation in the active
semantics tree. Widening to 800 restores all three. In Incular, the
missing placement previously became an origin offset but the child's
cached compositor layer remained attached. The retained paint, hit, and
semantic walks now use only placed inline children. The text annotation
does not speak the object-replacement marker.

The paired mounted presentation probe uses an explicit one-line cap.
The original empty spell-out registration at
`rendering/paragraph_test.dart:815` also passed, and the native
`flutter350_rich_paragraph_pronunciation` target verifies the mounted
empty paragraph has no pronunciation attributes or synthetic fragments.


## Unlimited line ellipsis

The original `rendering/paragraph_test.dart:755` now pairs with the same
100-pixel limit, a text-rendered inline child, and no line-count cap. An
ellipsized line is present but the child has no placeholder placement or
published semantic annotation. The retained semantics graph contains
only the paragraph's single spoken label.

The pinned `unlimited_ellipsis_wrap_reference_test.dart` separately tested
that explicit hard lines survive when they fit, the first over-wide hard
line is cut there, and ordinary word wrapping does not produce more
lines while unlimited ellipsis is active. The native text regression
checks the retained source bytes for all three. The earlier explicit
one-line variant still passes.

The authored reference also checks a three-pixel width, where the mark
itself cannot fit and no glyph is drawn, and a 4,096-character line, which
keeps only its first three glyphs at 45 pixels. The native stress
regression shapes at most four candidate prefixes instead of repeatedly
reshaping the entire hidden tail. It bounds the first candidate using
the already-measured clusters, then verifies the shaped ellipsis fits.
The same no-glyph check passes with or without an explicit one-line cap.
The reference's cluster-first cutoff is described in the
[pinned Flutter engine dependency](https://raw.githubusercontent.com/google/skia/0e5d8d1be81b22f5529a41cf4810dd89b63527ba/modules/skparagraph/src/TextLine.cpp).

## Inherited span scaling and overflow state

Four exact registrations in pinned `rendering/paragraph_test.dart`
passed: :488, :494, :500 and :538. The mounted native paragraph now
applies the supplied RichText text scaler in retained lowering.
Previously the scaler was discarded there, even though the standalone
text descriptor could shape scaled spans.

With Ahem, a 1.3× scale on nested 10-, 20-, 20- and 10-pixel spans now
produces a 78×26 paragraph rather than 60×20. The individual retained
cluster widths are 13, 26, 26 and 13, and natural font heights agree
within 0.0001 logical pixels. The 1× control keeps the unscaled values.
The original 100-pixel overflow comparison distinguishes naturally
wrapped text from a one-line cap or ellipsis; only the latter two
report truncation. Seven mounted layout and seven adjacent comparison
and semantics tests passed.

## Foreground-only updates

Pinned `rendering/paragraph_test.dart:507` passed. The retained native
comparison changes “Hello” to “Hello World”, which requires layout,
then changes only the literal color from black to white. It verifies
that the same retained cluster allocation survives the recolor, the
layout count does not increase, and the actual glyph commands turn
white. The second native case swaps colors in two adjacent spans,
collapses them to a single color and restores mixed colors without
running layout. Only the foreground map changes; the classifier does
not reuse layout for changed source, shape or background attributes.
Both focused cases and 17 neighboring rich layout and text paint
cases passed.

## Retained paragraph positions and tight selection boxes

Three exact pinned `rendering/paragraph_test.dart` registrations paired:
:129, :146 and :165. Native pointer mapping uses actual laid-out line
tops and shaped caret stops. Tight boxes are queried from the retained
glyph layout, with each selected run's natural baseline-aligned box.
The mixed-size original yields four boxes, including 60..120 by
3.75..8.75 for the smaller run. The standalone TextPainter and retained
widgets now share the same geometry and indexed font-run lookup.

The pinned `rich_paragraph_variable_line_hit_test.dart` control uses
10-, 30- and 10-pixel lines. Native selection at (25,25) previously
landed on byte 11 of the last line; it now lands on byte 3 of the middle
line, as Flutter does. The public read-only selection path also stores
the corrected caret. This authored control does not add original
registration credit.

The fixture uses the exact embedded FlutterTest face and an isolated
font collection, like the pinned engine's `RegisterTestFonts` (which
disables system fallback). Its missing ASCII apostrophe must not pick
a larger host glyph and shift the second line baseline. Normal
application font discovery is unchanged. Four focused and fourteen
adjacent retained-widget cases passed, along with the standalone
painter, bidi and selection-span suites. Non-tight height/width styles
and caret metrics are separate cases.

### Caret metrics and word boundaries

The exact pinned :99, :118 and :252 registrations passed. A mounted
native paragraph now exposes the local caret origin and full height
from the same geometry used by standalone TextPainter. The caret
advances between bytes 5 and 25 on one line, then reaches the next
line at byte 50. The size-10 test reports a 10-pixel caret at byte 5.
Word queries use the laid-out paragraph's original text: the three
probes return `polished`, one space, and `Queen's`, even though the
embedded test font has no glyph for the ASCII apostrophe. The seven
retained geometry, seven selection-pointer and 55 standalone painter
and bidi checks passed. The latter suite's previously ignored case
was not run or credited.
