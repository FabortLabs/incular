# Text residual pass

The existing findings handled in this pass were updated in place in their original groups:

- text-011, layout-003 and implicit-002: fixed (`rich-text`, `layout`, `animated-widgets`).
- text-010 and text-016: fixed (`text-editing`).
- text-005: deferred, with a migration plan (`text-editing`).
- text-003: blocked (`rich-text`).
- text-013: deferred (`rich-text`).

This file records only the new finding.

## text-019: fixed

- **Cause:** only `SelectableText` registered as a selectable, so plain `Text` under a `SelectionArea` was never collected, hit, painted or copied. In Flutter, `Text` builds a `RichText` whose `RenderParagraph` registers with `SelectionContainer.maybeOf(context)`. `SelectionContainer.disabled` provides no registrar, which opts its subtree out.
- **Change:**
  - `crates/incular-widgets/src/tree/text.rs`: the new `WidgetTree::is_selection_label` returns true for a `SelectableText`, or for a `Text` whose nearest selection boundary accepts children. Collection, hit lookup, area resolution and text value now accept such a `Text`.
  - `painting/behavior/content.rs`: `RenderKind::Text` paints its share of the selection below its glyphs.
  - `tree/focus.rs` (out of ownership): a registered `Text` can take focus from a selection gesture, but focus traversal skips it.
- **Test:** `crates/incular-widgets/tests/selection_area_lifetime.rs::plain_text_under_a_selection_area_is_selectable`. It covers selection across two paragraphs, the highlight, the `SelectionContainer.disabled` opt-out and a standalone `Text`.
- **Harness:** `text__selection_area_long_press_drag` matches with the Incular twin switched back to plain `Text` on both sides.
- **Status:** `accepted` in `findings/text.jsonl`, with `verified_by` set to "orchestrator fix pass".

## Validation

- **Scenarios:** baseline runs saved before any change, then rerun after. Saved in the scratch area as `out_before` and `out_after3`.

  | Prefix | Before | After |
  | --- | --- | --- |
  | `text__` | 12/27 | 17/27 |
  | `layout__` | 34/37 | 35/37 |
  | `implicit__` | 18/19 | 19/19 |
  | `transitions__` | 8/9 | 9/9 |

  No scenario regressed.
- **Tests:**
  - incular-text: `height_behavior`.
  - incular-widgets: `editable_text`, `editable_text_alignment`, `editable_text_bidi`, `editable_text_direction`, `editable_text_paragraph_layout`, `editable_text_interaction`, `editable_text_input_traits`, `selectable_text_pointer`, `selection_area_lifetime`, `text_height_behavior`, `focus_keyboard` and `tree_editing_semantics_gestures`.
  - incular-runtime: the text tests, `focus_mount` and `caret_on_screen`.
  - incular-controls: the text-field tests.
  - incular-material: `migrated_tests`.

  All of these pass.
- **Pre-existing failures** (edit-callback paths, not touched here; they fail with or without this pass's wrapper change):
  - `incular-text/tests/editing_copies.rs::a_selection_change_still_passes_through_edit_transforms`;
  - `incular-material/tests/length_formatting.rs` (all 6 tests);
  - `incular-controls/tests/w7_compound_controls.rs::autocomplete_semantic_set_text_reaches_editor`.
- **Clippy:** clean for the changed crates. The only warning is in `incular-navigation` (navigator.rs:979), which belongs to another worker.

## Out-of-ownership edits

- **incular-widgets:**
  - `tree/specs.rs`, `tree.rs`, `rendering/lowering/visual.rs` and `widget/structure.rs`: `content_padding` plumbing.
  - `tree/painting.rs`: import.
  - `tree/focus.rs`: text-019.
- **incular-runtime:** `frame.rs` and `frame/text_gestures.rs` now use the input clock (text-016).
- **incular-material:** `catalog/search.rs` sets `content_padding(8)`.
- **Harness twins:**
  - `harness/flutter/test/frames/text_test.dart`: the caret-blink probe was proven wrong.
  - `harness/incular/tests/frames/text.rs`: the selection-area twin is back to plain `Text`.

## Pixel follow-up



The text-011 metrics fix first painted glyphs on the unrounded baseline. That shifted text by 1 device px at DPR above 1 and regressed the text pixel scenes to about 2.4–4.3% of pixels over 16. Glyphs now paint on the line top plus the rounded ascent, as Flutter's raster does, and the reported baseline stays unrounded. The pixel scenes are back at the audit baseline and no scene is worse. Details are in `rich-text.md` under text-011.
