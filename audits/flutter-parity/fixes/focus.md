# Fix group: focus

Scenario run: `harness/run-frames.sh focus_input__` (18 scenarios). 17 match. `keyboard_page_scroll` now matches on offset; a residual on `row_focused` remains (see 003).

## focus_input-002 (P2): FocusScope default traversal. Outcome: fixed
- Cause: `FocusScope::new` installed `WidgetOrder`; Flutter inherits `ReadingOrderTraversalPolicy`.
- Change: `crates/incular-widgets/src/focus_keyboard.rs` uses `ReadingOrder` in `new()` and in a manual `Default`. `widget_order()` keeps the old order.
- Tests: `crates/incular-widgets/tests/focus_directional.rs` (reading-order default, widget-order opt-in).
- Harness: `focus_scope_default_traversal` matches.
- API: behaviour change. Callers that relied on construction order add `.widget_order()`.

## focus_input-003 (P1): PageDown on a focused list. Outcome: fixed, with a residual
- Cause: no default scroll shortcut outside text editing. Flutter maps PageUp/PageDown to a page `ScrollIntent` and Ctrl+arrows to a line `ScrollIntent`.
- Change: `frame.rs::run_default_focus_key` (non-text focus targets only). `tree/focus.rs::scroll_nearest_viewport` animates the nearest viewport: 0.8 viewport per page, 50 px per line, clamped to the scroll extent, 100 ms `EaseInOut`.
- Tests: `focus_directional.rs` (page target, Ctrl+arrow line, unconsumed without a viewport).
- Harness: offset matches on all frames. `row_focused` diverges from frame 6 (Flutter 0, Incular 1). Flutter drops focus from the autofocused row once it scrolls away. Incular keeps it. The cause is unverified, probably off-cache item disposal in the list, which these files do not own.
- Limitations: physics `shouldAcceptUserOffset` not checked.

## focus_input-004 (P1): arrow keys do not move focus. Outcome: fixed
- Cause: no geometric directional query; arrows drove only menus.
- Change: `incular-gestures/src/focus.rs` (`directional_focus_index`, Flutter's in-band and closest-edge search, scroll-owner preference, closed-loop wrap). `tree/focus.rs::directional_focus_target`. `frame.rs` routes plain arrows after the text-field check.
- Tests: `crates/incular-gestures/tests/focus_reading_order.rs` (nine tests), `focus_directional.rs`.
- Harness: `arrow_directional_focus` matches.
- Limitations: no hysteresis history; searches all tree focusables rather than the nearest scope; LTR only.

## focus_input-005 (P2): reading-order bands. Outcome: fixed
- Cause: a height-tolerance comparator, duplicated in `focus.rs` and `tree.rs` (the Tab path).
- Change: `incular-gestures/src/focus.rs::reading_order_by` implements Flutter's `_pickNext` band algorithm. Both copies call it. `tree.rs::sort_focus_members` is an out-of-ownership edit.
- Tests: `focus_reading_order.rs` (band overlap, out-of-band order, stable ties).
- Harness: `reading_order_band_overlap` matches.
- Limitations: LTR only.

## focus_input-006 (P2): ordered-policy ties. Outcome: fixed
- Cause: the ordered sort tie-broke by registration order. Flutter uses reading order as the secondary policy.
- Change: `tree.rs::sort_focus_members` (Ordered) and `focus.rs` (`OrderedTraversalPolicy`) sort by reading order first, then stably by order.
- Tests: none added. Covered by the scenario only.
- Harness: `ordered_unordered_reading_order` matches.

## focus_input-007 (P2): nested MouseRegion enter order. Outcome: fixed
- Cause: enters ran in hit order (frontmost first). Flutter's `MouseTracker` runs enters in reverse hit order.
- Change: `tree/raw_input.rs` iterates the entering set in reverse for mouse-region enter callbacks.
- Tests: `crates/incular-widgets/tests/mouse_region_enter_order.rs`.
- Harness: `mouse_region_nested_enter_order` matches.

## Validation
- `cargo test -p incular-gestures --test focus_reading_order` (9 pass). Existing `focus_scope_transitions` (5), `focus_highlight` (12), `gestures` (23) pass.
- `cargo test -p incular-widgets` on `focus_directional` (7), `mouse_region_enter_order` (1), `detector_hover` (7), `focus_keyboard` (3), `focus_traversal_group` (3), `action_focus` (6), `pointer_focus_wrappers` (20), `flutter_tier3_parity` (4). All pass.
- `cargo test -p incular-runtime`: `button_click_focus`, `text_field_scroll_keys`, `text_logical_arrows`, `text_shortcuts` (16), `selectable_text_shortcuts`, `initial_focus`, `focus_mount`, and `runtime` filtered by focus/keyboard/arrow/page/tab (38). All pass.
- `cargo clippy -p incular-gestures --all-targets -- -D warnings`: clean. Clippy on widgets and runtime reports no warnings in files this group touched.
- Not run: `cargo test -p incular-widgets --test scroll_pages_reorder` does not compile (scroll worker's `scroll_views.rs` and test). `incular-navigation` clippy fails under `-D warnings` (navigation worker's file).
