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
