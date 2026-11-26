# Editable text intrinsic measurements

Pinned `rendering/editable_intrinsics_test.dart:54,70,90` passed. The
native fixture registers the same Ahem font, uses the original 10-pixel
text, and reproduces the low-level reference's 1-pixel caret width.
The public widget's separate 2-pixel caret default is unchanged.

The measured maximum intrinsic width changes from 112 to 222 pixels
under 2× text scaling. The 2-pixel caret-and-gap reservation does not
scale. Five hard lines measure 50 pixels, then 10 when limited to one.
A forced 100-pixel strut changes the measured and mounted height from
10 to 100 without altering intrinsic width. All three changes preserve
the same render object. Measurement alone does not replace its layout
or viewport. Both options also feed the renderer's normal text layout,
not a test-only calculator.

All 15 focused editor measurement, alignment, and paragraph tests
passed. The original :12 additionally compares a detached renderer's
full diagnostic tree and minimum intrinsic width; it is not counted
by this batch.

## Explicit scaler and ambient changes

The pinned authored `editable_scaling_and_strut_test.dart` verifies
the widget-level precedence. Ambient 3× first gives a 30-pixel editor.
An explicit 2× scaler applies to both the glyphs and a forced 50-pixel
strut, giving 100-pixel height. Changing the ambient scale to 4× leaves
that override at 100; removing the override restores a 40-pixel field.
The native case also checks widths and retained identity through every
change. All four focused controls passed; the authored probe does not
add an original registration.

## Default line height when glyphs use a fallback font

The pinned authored `editable_default_strut_fallback_test.dart` uses
Ahem plus the repository's Inter fixture. A Greek glyph switches to
Inter; Flutter still keeps the editor at the base font's 20-pixel
line height. Incular previously expanded to 24 pixels. The default
editable strut now forces the base metrics, and the resolved text
layout reports that height instead of reintroducing the larger
pre-strut line metric. An explicit zero-size strut disables the
default and allows the natural fallback height. Both modes keep the
same render object.

Strut font metrics use a bounded cache. Repeated edits do not reshape
the template glyph, and registering a font invalidates stale entries.
The tests check both effects through native counters. All 62 distinct
focused editor, text painter and document checks passed. The authored
reference does not add an original registration.

## Minimum and maximum content widths

The pinned original `:12` and authored
`editable_minimum_width_test.dart` both passed. Native measurement uses
the text shaper's word boundaries and keeps the caret reservation out
of minimum width: five Ahem digits measure 50 minimum / 52 maximum;
`Hi World` measures 50 / 82, or 100 / 162 at 2×. This uses the same
font state as normal layout. Measuring after late font registration
does not suppress the retained editor's subsequent reflow.

The original is marked partial: native tests now cover its minimum
and maximum width and height, but not Dart's detached render object's
diagnostic tree. The authored probe adds no original registration.
