# Editable text custom painters

Reference: pinned Flutter 3.50.0, `rendering/editable_test.dart`,
the `custom painters` group. The native target is
`incular-widgets/tests/flutter350_editable_custom_painters.rs`.

The optional editors' slots record in viewport-local coordinates and
inside the existing text clip. The background comes after built-in
background highlights and caret, the text and its decorations come
next, and the foreground comes after any built-in foreground caret.
The test asserts the actual glyph and rectangle command order with
each slot singly, together, swapped, and sharing one painter.
Scrolling the text by 1000 preserves the painter's drawn coordinates
and supplies the changed text origin. An explicit editor repaint
invokes the installed painter.

A retained replacement asks its own repaint predicate. Returning
false preserves the picture; true repaints without laying out or
changing the editor's render object. Repaint notifier subscriptions
follow the currently installed delegates, and both slots sharing one
source request at most one repaint per notification. The existing
CustomPaint notification test also passes.

## Painter-only notifications

The original :1116 verifies that notifying a custom painter does not
call the parent editor's paint. The retained editor now replaces the
signaled auxiliary pass without invoking the editor paint, recording
glyphs again, or calling the other painter. If a full editor repaint
is independently pending, both painters run once in the new picture.
The native test verifies the editor paint counter and both painter
counters, and checks that adding or removing commands in either slot
keeps the other slot and real glyphs in order. Inside an isolated
antialias clip, a painter-only refresh preserves the clip layer ID and
advances its source generation so the renderer cannot reuse stale
pixels. The wgpu backend does not change.
