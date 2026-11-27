# Editable text width and line-width basis

Flutter's `EditableText.forceLine` defaults to true. Explicitly disabling it
makes the box track the width of the paragraph, keeping room for the caret
and the one-pixel trailing gap. Incular previously had no switch for this
behavior; a bounded parent always produced a full-width editor unless
someone manually set its size. Existing explicit sizes still win.

The authored pinned Flutter fixture and retained Rust test compare the same
single 10-pixel Ahem glyph. With parent bounds 100..500, the default width
is 500 and the natural width is **103**, not 100. The lower paragraph
constraint is 100 before adding the 3-pixel caret gutter; the upper bound
reserves it before layout. With a zero lower bound the result is 13, a
tight 130 bound still yields 130, and switching back restores 500 without
replacing the mounted editor or its controller. The retained fixture also
checks unbounded width and center alignment with a nonzero viewport offset.

The second authored comparison wraps `AA BB CC` into three Ahem lines under
a 50-pixel maximum. `TextWidthBasis::Parent` uses the unwrapped intrinsic
width capped at 50; `LongestLine` uses the 20-pixel wrapped line plus its
caret gutter for 23. With `force_line(true)`, either basis fills 50. Both
native layout backends match 50/23/50 with the same 30-pixel height. The
existing forced path still shapes once; the natural-width route reshapes
only if the measured width is not the final alignment width.

Pinned `rendering/editable_test.dart:2007`, “does not skip TextPainter.layout
because of invalid cache,” passes. Its default and shrunken sizing direction
is cross-checked in live Rust layout, not via a separate native dry layout
operation, so that original registration stays partial. The two authored
pinned widget cases and 18 focused Rust tests pass.
