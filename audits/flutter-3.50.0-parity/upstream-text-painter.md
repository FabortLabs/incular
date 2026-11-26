# Stateful text measurement and painting geometry

Ten pinned `painting/text_painter_test.dart` cases pair: explicit
direction, disposal and repeated disposal, word boundaries, plain text,
line-count updates and cached line lists, alignment offsets, longest-line
relayout, and fractional width. The Rust explicit errors correspond to
Flutter's invalid-state checks. The editor-facing word range still uses
UTF-8 bytes; the same second family emoji spans UTF-16 offsets 8–16.

`incular::text::TextPainter` measures spans through Incular's retained
TextEngine. Inline children contribute U+FFFC before and after measurement.
Geometry separates the shaped paragraph, reported content width, and
paint offset. The cached line list includes the applied offset without
copying glyphs. The painter skips redundant shaping at identical bounds
on the same font engine and invalidates reuse across font collections,
new widths, and configuration changes.

The pinned Ahem fixture verifies the 60-wide centered longest line with
left coordinate zero, all six alignments with retained metrics, a width
sequence 1,000→100→1,000, and an exact fractional line/intrinsic width
of 6.25. The retained tests also use the repository's Inter font without
needing an upstream checkout. This describes the measured and drawn
coordinates, not a complete Flutter TextPainter API or GPU pixel parity.

```sh
INCULAR_FLUTTER_AHEM_PATH="$PWD/.tmp/flutter/packages/flutter_tools/static/Ahem.ttf" \
  cargo test --locked --offline -j1 -p incular-text --test flutter350_text_painter --test flutter350_text_boundaries --test inline_placeholders -- --test-threads=1
```

## Caret affinity and grapheme edges

Six further upstream cases pair. The painter retains the snapped top
of each measured line. At an explicit newline, the logical source
position wins under both affinities; only a soft wrap disambiguates
the two sides of the same offset. The Ahem fixture sees the expected
positions at 0, 14, 28, and 42 px, and places the two carets on either
side of the 98 px wrap. A span with only children and then no text
has the pinned caret coordinates, including a clamped invalid offset.

The UTF-16 entry point uses the shape's measured grapheme boundaries
for family, skin-tone, flag, and other joined emoji. Internal UTF-16
positions do not become extra caret stops; the visible prefixes and
terminal offset agree with separately measured text. A deliberately
narrow Korean jamo layout remains queryable. The bidi-width regression
is partial because native Arabic fallback advances differ from the
Ahem fixture. What pairs there is the direction-sensitive caret-width
subtraction at the run boundary and the base-direction terminal
caret. The exact trailing-space/RTL registration ran only on Flutter.
All eight selected Flutter and 24 focused Rust cases passed.

## Scaled text and caret height

Ten additional pinned registrations pair. Plain and explicitly styled
spans scale once after inheritance; the painter preserves a distinct
explicit paragraph root size even when every visible character comes
from a differently styled child. With the pinned Ahem font, the default
14 px text height, 2× scaling, and 100 px root template agree. The 14 px
comparison selects Ahem in the Rust default style explicitly; it does
not require both platforms to ship the same system default font.
Static width and max intrinsic width use the same calculations as a
live painter under both finite and unbounded constraints.

At a run boundary, upstream and downstream affinity select the actual
run's caret height: 128→32 and 32→64 px. End-of-text returns to the
128 px root template under downstream affinity. A resolved 50 px strut
controls caret height without changing the source style, and an empty
zero-font paragraph still yields a finite template height. Ten Flutter
and 25 focused Rust cases passed (18 painter, 3 scaler, 4 line height).

## Full line metrics and height behavior

The original four-line fixture now matches hard and soft breaks, line
numbers, left edge, baseline, ascent, descent, unscaled ascent, height,
and widths 70 / 294 / 266 / 168. Flutter's default test face is the
engine's embedded **FlutterTest**, not the similarly monospaced Ahem:
their ascent is 75% and 80% of font size respectively. The Rust suite
extracts the exact pinned face without committing a font binary. The
308→294 discrepancy was a hanging space on the soft break; it remains
available to the caret but is excluded from the reported line width.
The nullable height override can be cleared and invalidates retained
layout. Mixed-size clipped paragraphs use the actual last retained
line's bottom, not the first line height multiplied by the count.

Both original Flutter registrations and 31 targeted Rust cases passed.
The hanging-space fix applies to line metrics; this does not assert
equivalence for every justify or arbitrary bidi layout.

```sh
python3 audits/flutter-3.50.0-parity/extract-flutter-test-font.py \
  .tmp/flutter .tmp/FlutterTest-embedded.ttf
INCULAR_FLUTTER_TEST_FONT_PATH="$PWD/.tmp/FlutterTest-embedded.ttf" \
INCULAR_FLUTTER_AHEM_PATH="$PWD/.tmp/flutter/packages/flutter_tools/static/Ahem.ttf" \
  cargo test --locked --offline -j1 -p incular-text --test flutter350_text_painter --test height_behavior --test selection_spans -- --test-threads=1
```
