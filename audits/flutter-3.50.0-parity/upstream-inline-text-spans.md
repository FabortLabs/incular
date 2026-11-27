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
Native screen-reader delivery, empty-vs-null Dart
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

## Inherited pronunciation and combined UTF-16 ranges

The exact pinned `TextSpan can compute StringAttributes` case passed.
Its five spans produce per-span spell-out and es-MX ranges in the
original order, including explicit opt-out and a descendant opt-in.
The combined run has exactly four annotations over `0..5`, `5..10`,
`5..10`, and `20..25`.

An authored reference and native control combine literal `ab` with
the spoken label `a🦀`, which has three UTF-16 units, then split around
an inline widget and a span with a semantics identifier. Pronunciation
offsets restart for each independent fragment, and a placeholder never
inherits pronunciation. Native storage borrows the locale while
building the fragments. Seven native span and 28 paragraph geometry
tests passed. Projecting independent fragments through the actual
accessibility tree is separate from this comparison.

### Mounted paragraph projection

An authored pinned frame confirms a rendered paragraph publishes the
spoken label `a🦀` with spell-out ranges `0..1` and `1..3` and locale
es-MX on `1..3`. The retained native paragraph now exposes that same
single text node, without altering its literal painted text `ab`.
Changing or clearing pronunciation keeps the semantic node identity;
a plain paragraph does not gain additional nodes or attributes.
Three focused native mounted pronunciation tests and three existing
inline-layout tests passed. Paragraphs requiring separate semantic
fragments keep the existing presentation fallback until their native
fragment nodes are implemented. This authored case adds no credit.

## Mouse cursor and enter/exit

The exact pinned `painting/text_span_test.dart:349` and `:390`
registrations passed. The focused Incular runtime fixture runs real
pointer input over the same three centered spans and observes the
resolved native-boundary cursor and enter/exit callbacks. A cursor is
recorded only over a retained shaped cluster belonging to that span;
plain children do not inherit it.

The authored pinned `text_span_mouse_order_and_clip_test.dart` and
native controls also verify that an enclosing mouse region supplies
the background cursor, its enter precedes the span's enter, and its
exit follows the span's exit. A displayed ellipsis takes no cursor from
the truncated span. Removing a hovered paragraph from under a still
pointer delivers its span's final exit, unlike the enclosing widget's
unmount policy. This does not implement the separate span gesture
recognizer API or its semantics actions. All five focused native cases
and 33 neighboring mouse routing/region/cursor cases passed.
