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
