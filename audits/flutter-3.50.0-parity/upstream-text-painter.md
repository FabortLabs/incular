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
