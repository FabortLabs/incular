# Unify shaped glyph storage

Status: proposed, higher-risk work after 00. Coordinate with 07/08.

## Finding and scope

`TextLine` in `crates/incular-text/src/engine.rs` has line-level glyph storage and
per-run arrays. Paragraph assembly also rebases related data. This is a duplication
hypothesis: coordinate spaces and cluster offsets must be audited before sharing.
Inspect `GlyphRun`/`GlyphPosition` in `crates/incular-rendering/src/glyphs.rs` and
WGPU lowering consumers; resolve the defining neutral owner if files have moved.

## Implementation sequence

1. Trace glyphs through shaping, hit testing, caret/selection, display lists and
   GPU lowering. Record every differing coordinate and byte-offset field.
2. Count unique backing allocations and copied bytes. Distinguish Arc sharing from
   actual copies, including cached paragraphs and assembled multiline layouts.
3. Design a shared immutable buffer with validated line/run ranges. Store exact
   translations/rebase offsets as metadata only where semantics permit.
4. Use checked range conversions. Do not impose shorter maximum text lengths or
   quantize positions to shrink fields; provide fallback for compact-offset limits.
5. Audit public TextLine/GlyphRun construction and access. Document compatibility
   or migration before changing supported fields; avoid a permanently duplicated
   compatibility buffer that erases the saving.
6. Migrate consumers in stages. Preserve backing/font lifetimes across eviction
   and renderer retention. Do not reconstruct the old arrays every frame.

## Evidence, validation and acceptance

Measure unique glyph bytes, allocations, range metadata and peak assembly memory
for short labels, long paragraphs, mixed fonts and editable text.

Use relevant engine, spans, selection_spans, editable_text_bidi,
editable_text_alignment and glyph_position tests. Cover ligatures, combining marks,
RTL/LTR boundaries, fallback fonts, wrapping, ellipsis, selection and affinity.
Require unchanged captures and editing behavior.

Accept demonstrated duplicate-buffer removal without extra shaping/rasterization,
precision loss, unsafe ranges or frame-time copying. Deliver ownership/coordinate
specification, API decision, phased patch and protocol-00 evidence. Revert if any
coordinate or lifecycle invariant cannot be preserved.
