# Inline text spans

Seven exact pinned Flutter 3.50.0 `painting/text_span_test.dart`
registrations passed: :102, :113, :125, :247, :266, :291, and :303.
The native text tree preserves the original literal/speech text choices,
including `abc`, `afooc`, and the U+FFFC between `b` and `c`. Spoken
labels and widget placeholders can be included or omitted independently
from the presentation flattening. Existing layout callers still see
literal text and stable placeholder byte positions.

Position comparison skips empty wrapper spans and returns the original
text or widget span, with exactly one UTF-16 unit per inline widget. In
Incular a widget span refers to its child by ID, so the child's own text
is not part of the parent's positional stream. The authored control
replays both affinities on ordinary and placeholder boundaries and
inside a surrogate pair; after the last text unit downstream returns
nothing while upstream resolves the preceding span.

The extracted span records preserve the exact literal string, semantics
label and identifier. Direct-child visitation preserves order, supports
short-circuiting and allows the test's original preorder index walk.
Five focused native span tests and 28 paragraph geometry controls passed.
Native screen-reader delivery, string attributes, empty-vs-null Dart
object representations, and mouse callbacks are
separate cases not credited here.

## Comparing replacements

Pinned originals :137 and :205 passed. The compared native
`RichParagraph` includes actual inline widget descriptors, not just its
placeholder size and ID. A change from width 10 to 11, a changed literal
widget or its text, a change to top alignment, or a height change deep
inside an inline child's own rich paragraph requires layout. A self or
structurally identical description returns identical. Native controls
also distinguish color-only paint from font-size relayout and keep
semantic-ID-only changes in a separate metadata category. The latter is
an Incular extension to this comparison, not an assertion of Dart's
semantics-ID invalidation rules. Three dedicated native tests and
three mounted rich-paragraph controls passed.
