# rich-text fix group

## text-001 (P1): RichText drops styles after the first run
- Outcome: fixed.
- Cause: the engine shaped a paragraph with one style; RichText flattened to the first run's style.
- Change: `layout_runs_with_options` in `crates/incular-text/src/engine.rs` pushes per-run ranged shaping properties; styled runs bypass the style-keyed shaping pool; per-span colors are recorded (`color_runs`) and painted per glyph run. Widget path: `descriptors.rs` (From<RichText>), `lowering/visual.rs`, `layout/behavior/text.rs`, `painting/behavior/content.rs`.
- Tests: `crates/incular-text/tests/rich_runs.rs` (5 tests, pass).
- Harness: `text__rich_span_style_metrics` now matches.
- Limitations: decorations and per-span shadows are not painted.

## text-003 (P2): WidgetSpan children are not laid out or painted
- Outcome: blocked.
- Cause: `WidgetSpan` has only an id and size; the widget tree has no child slot for inline placeholders. parley 0.12 supports inline boxes, so the engine side is not the blocker.
- Remaining work: engine InlineBox reservation with alignment; WidgetSpan children as Text render-object children, laid out at the reported box size, painted and hit-tested.
- Harness: `text__widget_span_layout` still diverges (220x19 vs 220x40).

**Text residual pass: still blocked.** parley 0.12 InlineBox support is confirmed. The remaining work: a widget-level `WidgetSpan(child)` that RichText lowers to element children; a placeholder list passed next to the runs to `layout_runs_with_options` (`StyledTextRun` is public, so no new field); `push_inline_box` with the measured child size and alignment; and the inline-box rects reported back so children are positioned, painted and hit-tested there. This did not fit the pass.

## text-004 (P2): DefaultTextStyle has no inherited maxLines
- Outcome: fixed.
- Change: `DefaultTextStyle` carries text_align, soft_wrap, overflow and max_lines; `Text` options are optional and inherit when unstated (`TextParagraphPolicy`). `Text::max_lines(None)` now inherits, as Flutter null does.
- Tests: existing `text_height_behavior`, `environment` and `implicit_animations` suites pass.
- Harness: `text__default_text_style_max_lines` matches. The diagnostic twin now calls `DefaultTextStyle::max_lines(1)`, the Flutter twin's setting.
- Limitations: nested DefaultTextStyle uses the nearest policy only.

## text-012 (P2): Text has no textScaler
- Outcome: fixed.
- Change: `Text::text_scaler`; applied to the base style and every span.
- Harness: `text__text_scaler_linear` matches. The diagnostic twin now uses 14 px with `text_scaler(TextScaler::linear(1.5))`.
- Limitations: the system MediaQuery text scale is not applied.

## text-013 (P2): Text has no strutStyle
- Outcome: deferred.
- Cause: `StrutStyle` is a type only; parley has no strut.
- Remaining work: `Text::strut_style` plus per-line minimum ascent and descent (or zero-width inline boxes), verified against a new Flutter scenario. Too large for this pass.

**Text residual pass: still deferred, now feasible.** After text-011 the engine works on unrounded `LineBox` values before snapping, so a strut is one more pass that widens each line box to the strut's ascent and descent, or replaces them when `force_strut_height` is set. What remains is the `Text::strut_style` plumbing through `WidgetKind::Text` (specs, descriptors, lowering, structure, `LayoutKey`) and a Flutter scenario.

## text-011 (P3), layout-003 (P3), implicit-002 (P3): line-metric rounding
- Outcome: blocked (one shared cause).
- Cause: parley quantizes ascent and descent to whole pixels (`BoxMetrics::from_font` quantize path). Flutter keeps whole-pixel paragraph heights but an unrounded baseline (11.625 at 12 px).
- Experiment: `ranged_builder(..., quantize = false)` fixed the baseline direction but broke heights (height behavior 19.14, scaler 25.41). Reverted.
- Remaining work: quantized heights with an unquantized baseline, via a parley patch or post-layout baseline correction from font metrics.
- Harness: all three residuals unchanged.

**Text residual pass: fixed.** parley rounded ascent and descent separately. SkParagraph rounds only each line's height and keeps the baseline unrounded (`InternalLineMetrics::height` and `baseline`). `engine.rs` now shapes unquantized and `snap_line_boxes` stacks the lines at whole-pixel heights with unrounded baselines; height-behavior trimming runs on the unrounded line boxes first. Test: `height_behavior.rs::line_heights_round_while_the_baseline_keeps_the_exact_ascent`. Now matching: `text__text_height_behavior_first_ascent`, `layout__baseline__cross_axis`, `implicit__default_text_style__text_style` and `transitions__text__default_style`. A before/after comparison of `text__`, `layout__`, `implicit__` and `transitions__` shows no new divergences.

**Pixel follow-up.** The first version painted glyphs on the unrounded baseline. That moved text by 1 device px at DPR 1.25, 1.5 and 2, and the text pixel scenes rose to 2.4–4.3% of pixels over 16. Row-band centroids showed the cause: Flutter's raster draws glyphs on the line top plus the *rounded* ascent (14 px Inter at 2x on row 2 * (8 + 14)). Only the reported baseline is unrounded. `snap_line_boxes` now moves the glyph offsets to top + round(ascent); `TextLine.baseline` and `metrics.baseline` keep 11.625. A one-run line still shares its glyph array with its paint run, and run origins are unchanged. After the fix, `run-pixels.sh --incular-only` shows text 0.32/0.26/0.17/0.16%, text_dark 0.27/0.19/0.21/0.15%, text_gray 0.05/0.03/0.01/0.02% and text_align 0.18/0.11/0.12/0.16%, the audit baseline. rich_text_spans improved to 0.02% at 1.25x and 2x, and no scene got worse. Frames: text__ 24/28, layout__ 35/37 (baseline__cross_axis still matches), implicit__ 19/19, transitions__ 9/9. Test: `height_behavior.rs::line_heights_round_while_the_baseline_keeps_the_exact_ascent` asserts a paint baseline of 12 and a reported baseline of 11.625.

## implicit-006 (P3): AnimatedDefaultTextStyle omits paragraph options
- Outcome: fixed.
- Change: `AnimatedDefaultTextStyle` gains text_align, soft_wrap, overflow and max_lines, passed through to the default-style scope.
- Limitations: textWidthBasis is not wired.

## Validation
- `cargo test -p incular-text --test rich_runs --test spans --test height_behavior --test style --test selection_spans`: all pass.
- `cargo test -p incular-widgets --test text_height_behavior --test environment --test implicit_animations --test editable_text_paragraph_layout --test text_shadow_alignment --test inherited_context_contracts`: all pass.
- `cargo check -p incular-widgets` (with and without `devtools`) and `cargo check -p incular-parity-harness --tests`: pass.
- Harness `text__`: 10/27 match (was 5/27); no scenario regressed against the pre-change report. `layout__baseline`, `implicit__default_text_style`, `transitions__`: no new divergences. `layout__`: 34/37 match; `layout__table__rows_and_columns` now diverges, but that scenario contains no text, so the cause is the concurrent table/layout edits, not this work (not isolated).
- Clippy was not run for the widgets crate in this pass.

## Shared-file edits
- `harness/incular/tests/frames/text.rs`: two diagnostic twins updated to the new API (`text_scaler`, `max_lines`), plus a TextScaler import and import formatting. The orchestrator should review.
- Other workers' concurrent edits in `constructors.rs`, `environment.rs`, `spans.rs` and `text.rs` were preserved; `spans.rs` `semantics_label` is intact.
