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

## Variable font axes during animation

Pinned original `painting/text_style_test.dart:743` and its
native comparison passed. Unspecified and empty lists keep their
distinction; exact endpoints keep order and duplicates. Matching
positions interpolate before divergent axes are reconciled by tag:
the last duplicate in each unmatched tail wins, and unmatched tags
appear or disappear at 0.5. The deterministic native tail order is
one of the orders allowed by the original assertions. The helper
accepts extrapolation and applies the signed 16.16 bound; because
Incular uses f32 for shaping, its largest representable positive
value below 32768 is slightly lower than Flutter's f64 bound.

Previously, `TextStyle` jumped straight from one variation setting
to the other at the midpoint, and the typed setter lost fractional
precision to one decimal place. Shaping now receives intervening
weights; raw settings in the ordinary quoted syntax are interpolated
too. Unsupported raw strings keep the previous discrete selection.
The default and identical settings do not allocate an axis list.

The authored native and Dart probes load the same pinned Roboto
Slab variable fixture, shape the same paragraph, and verify that
the widths at the tween samples equal the explicit intermediate
axis values in each engine. Width grows at the selected weights and
does not jump between 0.499 and 0.5. This verifies changed layout,
not identical cross-engine glyph rasterization. Twelve focused native
checks (including the fixture test) and two authored Flutter checks
passed.

To rerun both fixture checks from the repository root with the pinned
checkout present:

```sh
cargo test --locked -j 1 -p incular-text --test font_variations --test style -- --include-ignored --test-threads=1
export INCULAR_FLUTTER_VARIABLE_FONT="$PWD/.tmp/flutter/engine/src/flutter/testing/resources/RobotoSlab-VariableFont_wght.ttf"
bash audits/flutter-3.50.0-parity/run-reference-linux.sh probe font_variation_tween_test.dart
```

### Variable glyph masks and retained atlas entries

The layout fix revealed a second drift: Parley shaped the requested
face location, but Incular rasterized its default outlines and let
other weights alias those masks. The shaped run now carries the
face-normalized coordinates through paint to hinted and unhinted
wgpu glyph rasterization. Hinting state is bounded and includes the
axis location; the underlying parsed face is still shared. Varied
mask keys are exact, and the default instance uses the previous
compact key. Page eviction retires both kinds of identity.

The native fixture compares separate light, midpoint and heavy
masks for the same glyph at 56 ppem and the same subpixel phase.
Ink coverage increases at each sampled weight, the midpoint and an
explicit weight 500 share one entry, and the implicit and explicit
default weight share one entry. Warm requests reuse the masks,
parse the face only once, and retire and regenerate the correct
mask under budget pressure. The authored Flutter image test checks
the same coverage ordering and identical pixels between the
midpoint and explicit weight 500 in Flutter. This does not claim
identical antialiasing between the two engines.

Twenty-five focused native checks and the authored Flutter pixel
test passed. The pre-existing system-font mask comparison
`on_demand_masks_preserve_em_scale_baseline_and_antialiased_coverage`
also was run separately; it fails at the same glyph 135 coverage at
1× on the unmodified baseline `4bf4a279` and with this repair.
That comparison has not been weakened or counted as passing.

```sh
cargo test --locked -j 1 -p incular-wgpu --test variable_glyphs -- --include-ignored --test-threads=1
export INCULAR_FLUTTER_VARIABLE_FONT="$PWD/.tmp/flutter/engine/src/flutter/testing/resources/RobotoSlab-VariableFont_wght.ttf"
bash audits/flutter-3.50.0-parity/run-reference-linux.sh probe font_variation_pixels_test.dart
```
