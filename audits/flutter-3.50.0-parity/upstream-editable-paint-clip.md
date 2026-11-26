# Editable paint clips

Pinned `rendering/editable_test.dart:178` pairs. An overflowing
editor with `Clip.none` has no approximate paint clip. The native
`WidgetTree::editable_approximate_paint_clip` gives the editor's
local content bounds only when the corresponding paint clip applies.

The authored Flutter fixture records all four clip behaviors with
fitting and overflowing text. A fitting editor does not use a clip
or offscreen layer; overflowing text uses the requested clip, and only
the save-layer mode groups drawing before applying the antialiased
edge. The native fixture verifies the same modes on the *same* retained
editor, that overflowing glyph bounds extend outside the viewport
without an implicit clip when requested, and that overflow selection
and glyphs share one clip. The isolated clip keeps its compositor ID
and refreshes content when selection changes. The bare editor adds
no border. These tests inspect paint commands and isolation, not a
wgpu pixel readback.

The original :240 also passed in Flutter. It supplies a fixed
nonzero ViewportOffset to a fitting low-level RenderEditable and
therefore still expects a clip. That exact external-offset input
does not yet have a native paired fixture. The shared native query
does account for an actual nonzero editor scroll offset.

All 37 focused native cases passed after excluding one preexisting
stale semantics expectation (SetSelection while unfocused, confirmed
by rerunning the unmodified baseline). Both original Flutter
registrations and the authored clip probe passed.

## Editor focus and accessibility actions

The stale native test was corrected separately. A valid range remains
available to accessibility while unfocused, but Flutter only offers
SetSelection while the editor is focused and selection is enabled.
Read-only focused editors can still select; SetText additionally
requires an editable field. A pinned render-level Flutter fixture and
the matching native tree tests passed for focus, read-only and blur.
The full `editable_text` target passed all 17 cases. This was a
fixture correction, not a change to production semantics or audit
registration coverage.

## Full caret reveal at the horizontal edge

The bare left-aligned LTR single-line editor previously calculated a
clip using the caret margin but clamped scroll to the glyph-only
extent. In the native regression, the 4 px caret ended at x=12.946
in an 11.946 px viewport, while the offset remained zero. The
horizontal scroll range now uses the same glyph width, caret width,
and 1 px gap as clipping. Reveal moves 1 px to expose the whole
caret; page scroll uses the full 2 px extent. Document scrolling
also reveals the whole caret without changing the selection.

The authored pinned Flutter fixture used actual `EditableText`
and its scroll position, with a 1× device-pixel ratio to avoid
global pixel snapping. It verified both exact offsets and caret
edges. This is separate from the original :240 immutable external
offset case (still open). Right or center alignment was not
verified by this probe; the existing path for those is unchanged.

## Default and live clip mode

Pinned `rendering/editable_test.dart:80` and
`widgets/editable_text_test.dart:11497` passed. The native one-pixel
viewport retains all 10,000 characters and paints text on every
mode. The default and explicit hard-edge modes record a 1 px clip;
none records no clip; antialiasing records the right coverage mode;
only the save-layer mode has isolation. Updates keep the same
render object. The wider overflow/fit control also tests changing
the default live to antiAlias and back.

The isolated selection test now explicitly supplies a highlight,
preserving its actual selection-plus-glyph cache assertion after
the raw editor's default became unpainted. All five focused native
tests pass. The original :240 external offset case remains open.
