# Solid text paint precedence

Pinned `painting/text_style_test.dart:506` passed. All original solid
foreground combinations now have an equivalent visible result. Plain
colors blend continuously. If either endpoint supplies foreground paint,
the selected endpoint switches at 0.5; a plain endpoint is promoted to
solid paint. Merging or applying a plain color does not displace an
existing foreground paint. Applying a plain color without one now also
overrides an inherited style rather than silently keeping it.

Before the repair, Incular sometimes emitted its default glyph color
instead of the configured foreground. Retained Text, rich spans,
SelectableText and EditableText now select the effective foreground.
The new tests inspect the glyph commands, verify differently painted
inline spans, and show that a foreground-only update preserves the
render object and layout count. The authored Dart probe checks actual
opaque glyph pixels at six representative merge and tween states.
Forty-five focused Rust tests passed.

The original registration remains partial because Flutter exposes
nullable simple color and a general engine Paint object. Incular's
foreground descriptor is a solid color, not the full Paint API.
This does not compare Incular GPU pixels.

## Solid run backgrounds

Pinned `painting/text_style_test.dart:547,562` pass. Solid paint wins
over plain background color in both merge directions and apply. A
paint tween switches halfway, while simple colors interpolate and
may fade from or to absent. The :562 solid assertions pair; :547 is
partial because Flutter also checks its exact diagnostic strings.

The authored Flutter pixel probe checks a right-aligned 90-pixel
paragraph: the first Ahem line paints x=57–96 and the shorter
second line x=77–96, including the supplied origin. It checks the
top and bottom edges, no pixels outside, both midpoint endpoint
colors, and 40 pixels of coverage for a 20px run with height 2.
The pinned Skia implementation draws the measured run rectangle and
corrects the height using the run's font metrics:
[TextLine.cpp](https://raw.githubusercontent.com/google/skia/0e5d8d1be81b22f5529a41cf4810dd89b63527ba/modules/skparagraph/src/TextLine.cpp)
and [Run.cpp](https://raw.githubusercontent.com/google/skia/0e5d8d1be81b22f5529a41cf4810dd89b63527ba/modules/skparagraph/src/Run.cpp).

Incular's Text and rich paragraphs now emit solid background boxes
at their retained run advances. The native checks verify aligned
multi-line and partial-span geometry, explicit run height, default
and transparent paint suppression, and recoloring that preserves
the existing layout and renderer. Hard-break document composition
rebases the covered rectangles. A cache-key test and a retained
font test also confirm that a fallback-only style change now resolves
the new effective font instead of reusing a paint-only update.
Twenty-nine focused Rust tests and three authored Dart cases passed.
Complex overlapping grapheme boundaries, EditableText background
painting and final GPU pixel equivalence remain separate checks.
