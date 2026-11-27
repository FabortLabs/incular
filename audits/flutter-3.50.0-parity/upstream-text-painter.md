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
narrow Korean jamo layout remains queryable.

The two remaining caret registrations now pair against controlled font
metrics. Despite its name, :536 tests **trailing spaces**, not newlines.
On a 1,000 px line, five 14 px advances give the terminal LTR caret
at x=70 and the RTL caret with a 5 px caret width at x=925. The native
system-fallback control measures the lam advance independently and
checks that all four spaces still contribute their full width. For
:561, the mixed LTR/RTL string has caret x=0, 23, 28; the RTL run
boundary subtracts the caret width, but the paragraph's terminal edge
does not. Flutter's test face gives its missing lam a full-em
advance. The optional test-only font copy explicitly maps U+0644 to
the pinned face's same-advance square so Incular does not use a
different system fallback for the exact comparison. No production
font choice, text shaping, or editor default is changed.

Both originals and all 28 focused native painter tests passed:

```sh
python3 audits/flutter-3.50.0-parity/extract-flutter-test-font.py \
  .tmp/flutter .tmp/FlutterTest-embedded.ttf
# The optional fixture switch needs Python fontTools.
python3 audits/flutter-3.50.0-parity/extract-flutter-test-font.py \
  --arabic-lam-fixture .tmp/flutter .tmp/FlutterTest-lam.ttf
INCULAR_FLUTTER_TEST_FONT_PATH="$PWD/.tmp/FlutterTest-embedded.ttf" \
INCULAR_FLUTTER_TEST_LAM_FONT_PATH="$PWD/.tmp/FlutterTest-lam.ttf" \
INCULAR_FLUTTER_AHEM_PATH="$PWD/.tmp/flutter/packages/flutter_tools/static/Ahem.ttf" \
  cargo test --locked --offline -j1 -p incular-text --test flutter350_text_painter -- --test-threads=1
```

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

## Strut spacing and tall inline children

Three original strut registrations pair. An unstyled empty span takes
the explicit 100 px strut. Adding a 2× leading multiplier to the 10 px
strut adds exactly 10 px above and below, giving 120 px height and an
85 px alphabetic baseline. Preferred line height uses the actual
template height. A forced strut keeps a 1,000 px inline child in a 100 px
line without shrinking the child's own box. Three original Flutter and
31 focused Rust cases passed. Glyph selection boxes under forced strut
are a separate comparison.

## Line limit and hidden carets

Six authored Flutter 3.50.0 probes pair with native assertions. For
both LTR and RTL paragraphs, a line limit clips visual lines without
changing the source or its maximum intrinsic width. Hidden positions
map to the last retained line's paragraph-direction trailing edge,
with the RTL caret width subtracted. This works across soft wraps and
explicit line breaks. Dropping only the final empty line after a
newline does not report overflow. Clearing the limit invalidates the
layout and restores the omitted lines.

The probes ran alongside 28 targeted Rust tests. They are tracked
separately from the upstream registration count. This covers clipping
without ellipsis; custom ellipsis and tight selection geometry remain
separate work.

```sh
bash audits/flutter-3.50.0-parity/run-reference-linux.sh probe text_painter_line_limit_test.dart
```

## Tight selection bounds under leading and strut

Seven original cases pair. The new tight selection query uses retained
cluster advances and the resolved font's ascent and natural height
against each line's actual baseline. A 1 px glyph stays 1 px within a
20 px line; a 10 px glyph remains 10 px within a 1 px line, yielding
−4.5 px top and bottom insets. Clearing first and last leading makes
the same box exactly fill its line. Leaving the multiplier unset keeps
the glyph unchanged regardless of leading distribution. Forced 100 px
struts shift the baseline to 75 px (proportional) or 52.5 px (even) but
keep the selected glyph exactly 20 px high.

Seven pinned Flutter and 36 focused Rust tests passed. These cases use
tight boxes for complete glyphs; newline rectangles, additional box
width and height styles, and every mixed-direction selection are not
claimed.

## Inline widget dimensions and paint invalidation

Both pinned originals pass. The painter accepts one dimension record per
inline widget in source order. A changed height (30 px to 20 px on the
middle child) invalidates the existing paint geometry until the next
layout. Reassigning an identical slice preserves the same shaped layout
allocation; absent and empty values are also no-ops. Thirty focused
native tests passed, including a typed error for a mismatched number of
dimensions that leaves the previous layout untouched.

## Infinite minimums, finite paint, and centered trailing whitespace

Four original cases pair. An explicitly infinite minimum yields
infinite paragraph width, while the retained layout Size stays finite
by contract and returns a typed error for that request. Centered text
has no finite paint position and is skipped. Left-to-right justified
text still has a zero paint offset and can draw. Returning to finite
minimums yields the original natural width and centered offsets.

The unconstrained centered paragraph now aligns against its finite
natural width. With pinned Ahem, the original leading carets are
21, 35 and 49 pixels and the trailing caret clamps to the reported
420 px width. The paint entry point reports the specific missing
geometry error before layout, and dispatches valid shaped data with
the supplied origin afterwards. All 34 focused native tests passed
with the pinned Flutter Ahem font explicitly enabled.

## Clustered carets across scripts

Three original caret cases pair. The regression samples cover stacked
combining marks, Devanagari cluster boundaries, and upstream affinity at
the end of a Thai cluster. The first two compare each observable UTF-16
caret against independently shaped prefix widths and check that caret
positions do not reverse inside a cluster. The Thai terminal caret
reaches the measured width. All three exact pinned registrations passed.
This does not cover the registration for trailing breakable Unicode
spaces, where Parley still wraps the Ogham space at a tight width.

## Consecutive inline widget carets and wrap

Two original widget-span cases pair. A final caret after ordinary text
and an inline box reaches the measured paragraph width. In the 14-box
case, each touching box now owns its own replacement-character cell:
the cluster walker no longer consumes its neighbors when their edges
touch. At 500 px the first row ends after the sixth box; the downstream
caret at offset 18 starts the second row at x=0. All asserted caret and
box coordinates match pinned Ahem, including the 51 px-wide box in
the second row. Both pinned originals, 39 focused painter tests, and
4 inline-placeholder tests passed.

## Exact emoji and styled bidi carets

Three more pinned originals pair. Across all 23 UTF-16 positions in
the combined emoji string, the caret snaps to the 42, 98, and 126 px
grapheme edges (never to a UTF-16 surrogate or joiner midpoint).
Starting or ending a paragraph with an astronaut emoji next to a bold
script run produces the exact pinned caret arrays in LTR and RTL,
including the bidi boundary and terminal width.

The exact comparisons use the extracted Flutter tester font whose
test-only cmap maps the relevant unsupported scalars to its pinned
full-em missing-glyph advance, and the joiner to its zero-advance glyph.
This retains the original Unicode scalars and grapheme/bidi properties
while removing host emoji fallback from the numeric fixture. The
production font selection is unchanged. All three pinned originals and
all 42 focused native tests passed with the font fixtures enabled.

```sh
python3 audits/flutter-3.50.0-parity/extract-flutter-test-font.py \
  --emoji-caret-fixture .tmp/flutter .tmp/FlutterTest-emoji-caret.ttf
INCULAR_FLUTTER_EMOJI_CARET_FONT_PATH="$PWD/.tmp/FlutterTest-emoji-caret.ttf" \
  cargo test --locked --offline -j1 -p incular-text --test flutter350_text_painter emoji -- --test-threads=1
```

## Empty paragraphs, forced strut, and explicit natural height

Two pinned originals pair. StrutStyle can take its typographic
attributes from a TextStyle, leaving strut-only controls independent.
A forced 77 px strut still supplies the preferred and measured line
height for a glyph, an empty string and an empty span when both outer
leading edges are trimmed. Explicit natural height in a child cancels
the inherited 1,000× multiplier and produces a 10 px natural Ahem
line. Two pinned and all 44 focused native cases passed.
