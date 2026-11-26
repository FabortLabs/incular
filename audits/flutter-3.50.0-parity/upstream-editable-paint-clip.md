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
