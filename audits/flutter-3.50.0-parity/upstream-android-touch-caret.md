# Android finger drag and the caret

The pinned original `widgets/text_selection_test.dart:959` and native
Incular pointer sequence both start with an initially unset selection in
“I love flutter!”. A tap focuses the editor and collapses the caret at
offset 4. A subsequent single-finger drag moves the collapsed caret to
offset 10, rather than extending a selection from 4 to 10.

A supplementary neutral Flutter fixture confirms that a drag on an
unfocused field neither focuses it nor selects a range. A separate tap
takes focus. The older Incular regression assumed the first drag already
focused the field and that the second drag selected a range. Those
assumptions did not match the reference. The native regression now
verifies both the focus gate and the exact collapsed offsets.

This is a behavioral comparison on the Android runtime policy, not a
physical device or native keyboard test.

The [mounted builder follow-up](upstream-text-selection-gestures.md#drag-device-and-competing-renderer)
also verifies the original's fast timing: an immediate drag after a
completed tap moves the caret if the renderer won that tap, but a
builder-owned tap retains the separate double-tap-drag word behavior.
