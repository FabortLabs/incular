# EditableText caret reveal when keyboard insets grow

The original Flutter 3.50.0 registrations at
`widgets/editable_text_test.dart:496,542` passed. The native equivalents
use a retained SingleChildScrollView, a live focused editor, the same spacer
heights, and a host environment update. They check the outer controller on
the first inset frame, not after a settling loop.

The native editor previously cached only the editing value and selection
when deciding to reveal the caret. An increasing bottom view inset now
independently queues a reveal without animation. The authored pinned probe
normalizes the view to 800×600 at DPR 1: a 500-pixel keyboard inset moves the
controller directly to 5,920 with the selection unchanged. The native
viewport reaches the same exact extent; a later decrease does not reopen
a deliberately reset scroll position. The other original still reveals
the forward selection extent when the keyboard appears.

Two focused native fixes and eleven adjacent focus and viewport tests
passed; the other renderer and operating-system keyboard adapters were not
changed.
