# Text locale and accessibility identity

Four exact Flutter 3.50.0 originals in `widgets/text_semantics_test.dart`
passed on Linux:

- `:177` and `:196`: the retained text node for “Flutter 2050” preserves
  the `de-DE` tag both alone and inside SelectableRegion. Mobile
  publication preserves the exact UTF-16 range, and AccessKit receives
  the whole-label language rather than dropping it.
- `:231` and `:252`: the label and accessibility identifier belong to
  the same text node. When a route has its own annotation and requires
  explicit children, the text retains its identifier on a separate child
  rather than assigning it to the route. AccessKit publishes the text
  identifier as the author ID.

A native update control replaces locale and identifier, then clears
both without replacing the text node. It uses a supplementary character
to verify the 0..3 UTF-16 span. A partial pronunciation range is not
promoted to a node-wide language on desktop. Twenty-two focused native
tests passed. These checks inspect real adapter data, not the output of
an operating-system screen reader.

The separate rich-text identifier and recognizer-child identity originals
(`:142` and `:13`) are not covered by this slice.

## Spoken label replacement

The exact pinned `widgets/text_test.dart:428` passed. Both explicit
and inherited LTR produce just one leaf, “Double dollars,” with no
second semantic node for the visible “$$.” The native comparison also
updates the spoken label without changing retained glyph runs or the
semantic node's identity. The replacement does not reuse the visible
text's UTF-16 locale span. An independently attributed hint still
coexists with the locale on the original implicit text label.
Seventeen focused runtime and projection tests passed.
