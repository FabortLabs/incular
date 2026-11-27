# Editable selection endpoints and point equality

The pinned `rendering/editable_test.dart:872,891` originals pass.
A range covering only U+200E LEFT-TO-RIGHT MARK has no visible
selection boxes; the retained endpoint is at x=0 and has no direction.
It does not set the controller's selection or require focus. The public
point distinguishes coordinates and direction, and equal signed-zero
coordinates hash the same. Rust uses byte offsets and its own hashes.

The tight control uses real Ahem glyphs. A 5px fixed viewport scroll
and a 30×25 ancestor translation leave the local endpoints at (-5,10)
and (15,10); moving the caret between the letters reports (5,10) with
no direction. Reversing the selection preserves the logical start
and end geometry. This comparison is independent of touch-handle
artwork and the global screen-space IME caret.

Three direct and 14 adjacent native bidi and viewport checks passed.
