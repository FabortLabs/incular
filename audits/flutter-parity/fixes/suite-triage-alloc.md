# suite-triage-alloc

Triage of the allocation-budget tests that the parity fix pass broke. These are
suite failures, not audit findings, so there is no `.jsonl` record.

Method: a temporary profiling test (removed afterwards) installed an allocator
that captured a backtrace for every allocation during measured frames. It
aggregated the allocations by their first Incular frame and was run on both the
baseline worktree (`base-e25e45bc`) and the branch, and the two outputs were
diffed.

## Results

| test | budget | baseline | branch before | branch after |
| --- | --- | --- | --- | --- |
| `reconciliation_allocations::scrolling_list_builder_allocations_per_frame` | < 680 | 630 | 723 (fail) | 648 (pass) |
| `reconciliation_allocations::scrolling_list_builder_in_layout_builder_allocations_per_frame` | < 680 | 630 | 723 (fail) | 648 (pass) |
| `text_field_allocations::resize_frame_allocations_stay_bounded_for_many_text_fields` | < 600 | 309 | 612 (fail) | 311 (pass) |

No budget was raised.

## Cause 1: `ShowOnScreen` pushed on every visited semantic element (real regression)

The semantics fixes (`semantics.md` / `semantics-2.md`) made `collect_semantics_inner`
add `SemanticActionKind::ShowOnScreen` to every element below a scroll view. That
is Flutter's behavior: every node inside a viewport advertises showOnScreen. The
push ran for every element the walk visited, including role-less ones whose
action list is then thrown away. Each such push allocated a fresh `Vec`. Nodes
that do have a role and an exact-capacity action list (Button `vec![..]`,
TextField, `explicit.actions.clone()`) reallocated instead.

- Text fields: about 6 extra allocations per field per frame (+300 per frame).
- List: about 1 extra allocation per visible button row.

Changes, all in `crates/incular-widgets/src/tree/semantics.rs` (behavior unchanged):
- Add `ShowOnScreen` only when `role.is_some()`, which means only to nodes that are
  emitted. The `has_scroll_ancestor` walk now runs last in the condition.
- New `actions_with_room` builds action lists with one spare slot. It is used by
  the Button, SelectableText, TextField and explicit-semantics arms. The Gesture
  arm's capacity went from 2 to 3. This stops the reallocation caused by the
  `ShowOnScreen` push.

## Cause 2: per-layout scratch vectors in the text engine (partly removed)

The text fixes (`text-3.md`, `rich-text.md`) added these per-layout steps:
- `style_properties`, which returns a `Vec`;
- `snap_line_boxes`, which uses a `shifts` scratch `Vec`;
- `line_boxes`;
- `color_runs`, which allocates an `Arc` even when it is empty;
- glyph arrays re-shifted onto whole-pixel baselines.

These run only on a layout-cache miss, which happens about 1.5 times per frame in
the scrolling test.

Changes in `crates/incular-text/src/engine.rs` (behavior unchanged):
- `style_properties` became `for_each_style_property`, which pushes straight into
  the Parley builder. The returned `Vec` is gone.
- `snap_line_boxes` computes each line's (reported baseline, paint baseline)
  lazily with `scan`. The scratch `Vec` is gone. The total height is folded in
  the same order, so the result is bit-identical.

Remaining and inherent, about 5 per text-layout miss:
- the `line_boxes` copy and the empty `color_runs` `Arc`;
- the re-shifted glyph `Arc`s. SkParagraph places glyphs on a whole-pixel
  baseline while the reported baseline keeps its fraction (see the
  `snap_line_boxes` doc and the `text@1.25` / `text@2.0` pixel scenes), so the
  shifted glyphs need new arrays.

Small remaining deltas, each 1 to 2 per frame:
- clip stage creation (`create_clip_stage`, `RenderLayers` clone);
- extra inherited-dependency subscriptions from the per-aspect MediaQuery keys;
- `sort_sibling_group` key vectors.

Altogether the scrolling tests sit at about +18 per frame over the baseline, inside
the existing 680 budget.

## Validation

- `cargo test -p incular --test reconciliation_allocations --test text_field_allocations`: 3/3 pass.
- `cargo test -p incular-text --test engine --test height_behavior --test inline_placeholders --test document_layout --test selection_spans`: all pass.
- `cargo test -p incular-widgets --test semantics_flutter_parity`: 14/14 pass.
- `cargo test -p incular-runtime --test semantic_action_dispatch`: 15/15 pass.
- `cargo clippy -p incular-text -p incular-widgets --all-targets -- -D warnings`: clean.

Harness:
- `text__`: 24/28 match. The 4 that diverge are editing-behavior scenarios
  (`editing_selection_offsets`, `editable_undo_redo`,
  `editable_enter_callback_order`, `editable_undo_no_programmatic_selection`),
  which differ in text length and callback counts. They are not layout or metric
  results, and these changes do not touch them.
- `semantics__`: 18/19 match. `semantics__enabled_default` diverges on
  `enabled_state`, which these changes do not touch either.
