# focus_input: Focus and input

## Summary

Revision a19bf487 (resumed after the clean stop). 18 frame scenarios: 12 match, 6 diverge. Divergences: FocusScope default policy (002), PageDown on a focused list (003), arrow-key directional focus missing (004), reading-order bands (005), ordered-policy ties (006), nested MouseRegion enter order (007). Finding 001 (childless SizedBox claims hits) is rejected: the scenario matches with taps=1 on both sides.

## Scenario results

| Scenario | Result | Notes |
| --- | --- | --- |
| focus_scope_default_traversal | diverges frame 2 | 002 |
| shortcuts_action_intent | match | |
| keyboard_page_scroll | diverges frame 3 | 003 |
| mouse_region_enter_hover_exit | match | |
| listener_pointer_lifecycle | match | |
| ignore_pointer_falls_through | match | |
| absorb_pointer_blocks_sibling | match | |
| childless_sizedbox_passes_hits | match | 001 rejected (taps 1/1) |
| reading_order_band_overlap | diverges frame 2 | 005 |
| ordered_unordered_reading_order | diverges frame 3 | 006 |
| key_event_bubbles_past_ignored | match | inner 2, outer 2 |
| mouse_region_opaque_blocks_underlay | match | |
| mouse_region_translucent_passes_underlay | match | |
| mouse_region_nested_enter_order | diverges frame 1 | 007 |
| tap_region_group_outside | match | group and outside counts |
| arrow_directional_focus | diverges frame 2 | 004 |
| shift_tab_reverse_traversal | match | Shift+Tab reverses |
| shortcut_chord_requires_modifiers | match | Ctrl+K and K separated |

No frame-0 twin mismatches were found.

## Findings

- focus_input-001 (P1): rejected. Childless SizedBox over a tappable sibling leaves the tap on the sibling on both sides.
- focus_input-002 (P2, pending-review): FocusScope default policy is widget order; Flutter uses reading order.
- focus_input-003 (P1, pending-review): PageDown does not scroll a focused list.
- focus_input-004 (P1, pending-review): arrow keys do not move focus directionally.
- focus_input-005 (P2, pending-review): reading order uses height tolerance instead of band overlap.
- focus_input-006 (P2, pending-review): ordered traversal ties use registration order.
- focus_input-007 (P2, pending-review): nested MouseRegion enter callbacks fire inner first.

## Coverage

Coverage file: 205 rows. Every row has a status: 183 checked (scenario, partial, reading, missing or out-of-scope) and 23 unchecked with a reason. The unchecked rows are reassigned layout, text, controls, menus and scroll rows, plus lock-key (LockState, KeyboardLockMode) and key-repeat (KeyRepeatEvent) rows, which the harness cannot send.

TapRegion: the inventory rows for tap_region.dart (TapRegion, TapRegionSurface, RenderTapRegion, RenderTapRegionSurface, TapRegionRegistry) are in misc.jsonl and TextFieldTapRegion in text.jsonl. No row was added. Outside-tap group behaviour is covered by tap_region_group_outside (match). Consume-outside-taps is not compared.

## Harness requests

- Lock-key presses (LockState, KeyboardLockMode).
- Key repeat events (KeyRepeatEvent).

## Open questions

- KeyEventResult skipRemainingHandlers has no Incular equivalent.
- UnfocusDisposition has no Incular counterpart.
- Autofocus precedence: Flutter uses mount order; Incular uses traversal order.
- TraversalEdgeBehavior other than closed loop is absent.
- Focus listener notification order was reasoned from Flutter's dirty-node order and matches Incular for a sibling move; not frame-compared.

## Validation run

- `harness/run-frames.sh focus_input__`: 18 scenarios, report at harness/out/frames/report-focus_input__.md.
- `flutter analyze` on focus_input_test.dart: no issues. `flutter analyze test/frames`: clean.
- `cargo check -q -p incular-parity-harness --tests`: passes.
- Coverage JSONL and findings JSONL parse as JSON.
