# Selection handle drags through a degenerate editor

Both pinned `widgets/text_selection_test.dart:1275,1359` originals
pass. The native interaction begins with an actual focused field and
visible end handle, accepts a touch drag, and changes the retained
widget while the contact stays down. One case uses the same `1e-310`
Y scale, whose inverse exceeds finite `f64`; the other uses a text
scale of zero and verifies the measured line height is zero.

Before the native fix, raw hit lookup invented end-of-document or
zero offsets for those layouts. The drag now ignores an unmeasurable
sample without losing its capture, moving either selection endpoint,
or changing focus. Restoring normal geometry resumes the same drag
and preserves the committed text.

The authored pinned Android comparison and native finite control
keep the editor at a live 2× Y scale around the original handle edge.
A move of 1.4 old row heights stays on the original row. A move of
2.1 old row heights advances exactly one row at the same column.
Both keep the fixed endpoint and focus. The runtime computes the
snap from the editor's current local transform and line height,
not the global height saved before the gesture crossed touch slop.

Three direct and 21 adjacent native selection, reveal and editing
geometry checks passed.
