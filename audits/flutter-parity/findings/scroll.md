# scroll: Scroll positions, views, and scrolling widgets

Revalidated at HEAD `a19bf487`. Earlier findings were recorded at `a425aa2e`; `git diff a425aa2e HEAD` for the scroll paths touched only `crates/incular-scroll/src/extent_index.rs` and `crates/incular-widgets/src/scrolling/layout.rs`. Neither touches the jump, notification, or page-controller code cited here, so all five earlier findings still reproduce.

## Summary

Five paired frame twins now cover controller jumps (in range and out of range), jump notifications, animation with retargeting, animated-scroll notifications, and a fractional PageView with a nonzero initial page. Animated scrolls match Flutter, including start, update, and end notifications. Programmatic jumps diverge in two ways: an out-of-range jump is clamped instead of springing back (scroll-001), and changed jumps emit no start or end notification (scroll-005). Source review adds missing APIs (PageController page navigation, initial scroll offset, `ensureVisible`), a missing PageView `onPageChanged` callback (scroll-007), and missing keyboard scrolling for scroll views (scroll-006). The coverage inventory has 104 rows: 11 partial-scenario, 40 partial-reading, 12 out-of-scope, 7 missing, and 34 unchecked with a stated reason. Most of the unchecked rows are render-object internals owned by the slivers package or delegate and state classes, plus a few undiscovered counterparts; they were not compared to Flutter in this run.

## Scenarios

Run: `harness/run-frames.sh scroll__` (Flutter 3.41.7 and Incular at a19bf487): 5 scroll scenarios, plus 2 physics scenarios that the filter also matched.

| scenario | result | first divergence |
| --- | --- | --- |
| `scroll__controller__jump_clamp` | DIVERGES | frame 3 (48 ms), `value.offset` Flutter 100000, Incular 1400 (scroll-001) |
| `scroll__controller__jump_activity_notifications` | DIVERGES | frame 2 (32 ms), `notifications.start` and `notifications.end` Flutter 1, Incular 0 (scroll-005) |
| `scroll__controller__animate_retarget` | match | none |
| `scroll__controller__animate_activity_notifications` (new) | match | none; start 1, update 7 by frame 8, end not yet reached in 8 frames on both sides |
| `scroll__page_view__fraction_initial_page` | match | none |
| `physics__bouncing_scroll__extent_handoff`, `physics__clamping_scroll__spline` (physics package) | match | none |

Frame 0 layout matched in every scroll scenario. The new notification scenario is a positive control: it records nonzero counts on both sides, so the animated-path match is not two zeros.

## Coverage

Each inventory row in `audits/flutter-parity/coverage/scroll.jsonl` has a status and a coverage sentence. Status counts: scenario-backed partial (partial-scenario) 11, partial-reading 40, out-of-scope 12, missing 7, unchecked 34. No row is marked `scenario` or `scenario+reading` in full, because no row was compared end to end.

Main checked areas:

| Flutter item | Method | Result |
| --- | --- | --- |
| `ScrollController.jumpTo` | scenarios jump_clamp, jump_activity_notifications | in-range matches; out-of-range (scroll-001) and notifications (scroll-005) diverge |
| `ScrollController.animateTo` | scenarios animate_retarget, animate_activity_notifications | match |
| `ScrollController` attach/detach, multiple positions, keepScrollOffset, onAttach/onDetach | not checked | Incular holds one live metric attachment; the multi-position model was not compared |
| `ScrollNotification` order for programmatic scrolls | two notification scenarios | jump diverges; animation matches. Drag, fling and user-scroll order not checked (no pointer scenario harness) |
| `PageView` / `PageController` geometry | scenario fraction_initial_page | match (via deferred jump) |
| `PageView` `onPageChanged` | source | missing (scroll-007) |
| Scroll physics types | constants and builders read; two physics scenarios from the physics package | builders present; no new physics scenario |
| `ScrollBehavior.getScrollPhysics` platform defaults | source (`crates/incular-widgets/src/scroll_behavior.rs:171-186` vs Flutter `scroll_configuration.dart:243-257`) | platform mapping matches; parent chaining not compared |
| Keyboard scrolling via `ScrollIntent` / `ScrollAction` | source and rg | missing (scroll-006) |
| `Scrollable.ensureVisible` and `ScrollPositionAlignmentPolicy` | rg | missing (scroll-004) |
| `RawScrollbar`, overscroll indicators, `ListWheelScrollView`, `NestedScrollView`, `DraggableScrollableSheet` | source location and existing Incular tests only | partial-reading; no frame or pixel comparison |

## Findings

### scroll-001 — Out-of-range jump is clamped instead of springing back

- Severity: P2. Category: behavior. Impact class: visible-motion.
- Flutter: `widgets/scroll_position_with_single_context.dart:197-207` (`jumpTo` calls `forcePixels`, then `goBallistic(0.0)`).
- Incular: `crates/incular-scroll/src/controller.rs:246-263` clamps the requested value to `[0, max_offset]`.
- Evidence: `scroll__controller__jump_clamp`, frame 3 (48 ms) after `jumpTo(100000)`: Flutter offset `100000`, Incular `1400`. Flutter then springs back (about `97856` at 64 ms). Revalidated at HEAD.
- Impact: a programmatic jump past the extent lands on the edge at once in Incular, and the intermediate motion under bouncing physics is different.
- Likely cause: `jump_to` treats the target as a bounded assignment.
- Fix direction: keep the requested offset and start the ballistic correction as Flutter does.
- Confidence: high. Limitation: one out-of-range sample with default ListView physics.

### scroll-002 — PageController page-based navigation API is missing

- Severity: P2. Category: missing-api. Impact class: missing-feature.
- Flutter: `widgets/page_view.dart:119-125, 174-226`.
- Incular: `crates/incular-widgets/src/scrolling/scroll_views.rs:1036` (`pub type PageController = ScrollController`).
- Evidence: source reading and rg. No initialPage, page getter, jumpToPage or animateToPage. The paired scenario uses `deferred_jump_to(400)` as a stand-in.
- Impact: page-number callers must compute pixel offsets.
- Fix direction: page-coordinate configuration and operations using viewport fraction and viewport extent.
- Confidence: high.

### scroll-003 — ScrollController cannot configure an initial offset

- Severity: P2. Category: missing-api. Impact class: missing-feature.
- Flutter: `widgets/scroll_controller.dart:74-95`.
- Incular: `crates/incular-scroll/src/controller.rs:102` (`ScrollController::new` takes no arguments).
- Evidence: source reading. Only `jump_to` and `deferred_jump_to` set an initial position; restoration is the only construction-time offset source.
- Fix direction: an initial-offset option applied on the first extent publication, with restoration taking Flutter-compatible precedence.
- Confidence: high. Limitation: the precedence rule was not exercised.

### scroll-004 — Scrollable.ensureVisible has no Incular equivalent

- Severity: P2. Category: missing-api. Impact class: missing-feature.
- Flutter: `widgets/scrollable.dart:491-531`.
- Incular: MISSING. rg for `ensure_visible`, `ensureVisible` and `ScrollPositionAlignmentPolicy` in `crates/incular-widgets` and `crates/incular-scroll` finds only `raw_tooltip.rs:253`, which is unrelated.
- Fix direction: a tree-aware operation that walks ancestor viewports with Flutter alignment and animation policy.
- Confidence: high.

### scroll-005 — Programmatic jumps omit start and end notifications

- Severity: P0. Category: behavior. Impact class: lost-or-duplicate-callback.
- Flutter: `widgets/scroll_position_with_single_context.dart:197-206` (`didStartScroll`, `didUpdateScrollPositionBy`, `didEndScroll`).
- Incular: `crates/incular-scroll/src/controller.rs:246-266` dispatches only an Update notification.
- Evidence: `scroll__controller__jump_activity_notifications`, frame 2 (32 ms) after `jumpTo(125)`: Flutter start 1 and end 1, Incular 0 and 0, both with one update. Independently rerun at HEAD. The animated twin (`scroll__controller__animate_activity_notifications`) matches, which localizes the gap to the jump path.
- Impact: listeners that track activity (for example, to pause work during scrolling) miss both events for jumps.
- Likely cause: `jump_to` bypasses the activity bracket.
- Fix direction: emit Start, Update and End around a changed jump, in Flutter's order.
- Confidence: high. Limitation: observation points differ (Incular controller listener versus Flutter NotificationListener).

### scroll-006 — Scroll views have no keyboard scrolling

- Severity: P1. Category: behavior. Impact class: missing-feature.
- Flutter: `widgets/app.dart:1283-1330` (default ScrollIntent bindings for arrow, Ctrl+arrow, PageUp/PageDown); `widgets/scrollable_helpers.dart:377-460` (ScrollIntent, ScrollAction, page increment 80%).
- Incular: `crates/incular-runtime/src/frame.rs:1953-1961` routes arrow keys only to menu directional focus; `crates/incular-widgets/src/tree/focus.rs:217-240` is the same menu logic. PageUp and PageDown scroll only text editing (`crates/incular-runtime/src/text_shortcuts.rs`).
- Evidence: source reading and rg. No ScrollIntent, ScrollAction or ScrollIncrementType counterpart exists.
- Impact: a focused ListView, GridView or CustomScrollView cannot be scrolled from the keyboard without application code, which is a common desktop and web expectation.
- Likely cause: no scroll intent or action is defined for scroll views.
- Fix direction: ScrollIntent/ScrollAction equivalents with line and page increments and default bindings.
- Confidence: medium. Limitations: application key handlers calling `jump_to` could cover this; the frame harness has no keyboard input capability, so no scenario was run.

### scroll-007 — PageView has no onPageChanged callback

- Severity: P1. Category: behavior. Impact class: missing-feature.
- Flutter: `widgets/page_view.dart:844` (field) and `940-958` (depth-0 ScrollUpdateNotification handler with `_lastReportedPage` guard).
- Incular: `crates/incular-widgets/src/scrolling/scroll_views.rs:1056-1080` (PageView has no callback field). The only `on_page_changed` in the repository is `PaginatedDataTable` (`crates/incular-material/src/catalog/data_table.rs:669`).
- Evidence: source reading and rg.
- Impact: applications cannot react to page changes (indicators, lazy content) without polling the controller.
- Fix direction: an `on_page_changed` callback fired once per rounded page change from depth-0 update notifications.
- Confidence: high. Limitation: timing was not compared with a frame scenario.

### scroll-008 — No ScrollAwareImageProvider equivalent

- Severity: P3. Category: behavior. Impact class: missing-feature. Reassigned to the image package for the decode path.
- Flutter: `widgets/scroll_aware_image_provider.dart:48`.
- Incular: MISSING. rg for `scroll_aware` and `ScrollAware` in `crates` found nothing.
- Evidence: source reading and rg.
- Impact: images are not deferred during fast scrolling; this may cost frames in image-heavy lists.
- Fix direction: a provider wrapper that consults scroll activity and resolves once idle; the image package owner should review.
- Confidence: medium. Limitation: not measured.

## Pixel scene proposals

No pixel scene was added. `RawScrollbar` fade and thumb geometry, the glowing and stretching overscroll indicators, and DraggableScrollableSheet are paint-visible candidates for the orchestrator's pixel batch. The current pixel harness is static and cannot verify fade timing.

## Harness requests

1. A pointer drag and fling input capability for the frame harness. It is needed to verify drag, fling and user-scroll notification order and the hold/idle activity states.
2. A keyboard input capability (key down events routed to the focused scroll view). It is needed for a scroll-006 twin with Flutter's default shortcuts.
3. Optional: an accessor for PageView callbacks (after scroll-007 is fixed) so `onPageChanged` ordering can be recorded.

## Open questions

- Whether `SliverOverlapAbsorberHandle` (no Incular counterpart found) is needed; overlap is communicated through the sliver descriptor instead. It may be an API-shape difference only. Not yet tested.
- Multi-position attachment: Incular's explicit single live metric attachment is a deliberate contract, so Flutter's multi-position `ScrollController` behaviour is not reported as a finding here.
- Unchecked rows (34) are: render-object internals owned by the slivers package (`RenderSliverFill*`, `OverScrollHeaderStretchConfiguration`, `RenderSliverScrollingPersistentHeader`), the child delegates (reassigned to slivers), two-dimensional and list-wheel child delegates (counterparts not yet searched), hold/idle/activity-delegate states (names absent from Incular), `ScrollNotificationObserver`, `NestedScrollView` viewport render objects, and a few data classes. Each row says what is missing.

## Validation run

- `harness/run-frames.sh scroll__` at a19bf487: 5 scroll scenarios plus the 2 physics scenarios; results above. Report at `harness/out/frames/report-scroll__.md`.
- `cargo check -q -p incular-parity-harness --tests`: passed (warnings only from other workers' `text.rs`).
- `rustfmt --edition 2024 harness/incular/tests/frames/scroll.rs`: applied.
- `flutter analyze test/frames/scroll_test.dart`: no issues.
- `pwsh -NoProfile -File audits/flutter-parity/inventory_tool/render-inventory.ps1`: exit 0 with all coverage files valid.
- Not run: the cargo test-constrained suite and clippy (out of this worker's scope).


## Status at clean stop

Stopped on request before the requested corrections were finished. Frame suite `scroll__` was run once more before the stop; no further suites run.

Done in this pass:
- Added Flutter and Incular twins for drag, fling, clamping and bouncing overscroll, PageView fling, ListWheel selected-fling, DraggableScrollableSheet drag extent, NestedScrollView outer and inner drag offsets, and PageStorage keep-offset (single child). Both files compile and analyze clean.
- The `scroll__` run reported divergences for drag, fling, overscroll, PageView fling, ListWheel, draggable sheet and NestedScrollView. These are observed harness results, NOT yet verified or recorded as findings. Some are likely twin mismatches (for example, the ListWheel frame-0 selection callback and a NestedScrollView body that is not an independent inner scroll). Treat them as unreviewed.
- The keep-offset PageStorage twin matches.

Not done:
- scroll-006 is still pending; it duplicates focus_input-003 (PageDown does not scroll a focused list) and should be merged or rejected with a cross-reference.
- scroll-007 is still P1; the Incular listener workaround (controller notification listener on page changes) exists in the new PageView twin, so it should be downgraded to P2 after review.
- RawScrollbar thumb geometry and fade timing: no twin. Thumb is paint-only, so this needs a pixel scene.
- ScrollController attach/detach beyond the keep-offset twin, and multi-position behavior: not checked.
- UNRESOLVED coverage rows were not searched; they are reset to unchecked with reason "clean stop before audit".

## Resumed analysis and final status (supersedes the clean-stop notes above)

Run at a19bf487: `harness/run-frames.sh scroll__` gives 7 of 17 scenarios matching (physics scenarios included in the filter). The new twins are analysed below; none of the divergences is a harness twin mismatch once the checks here are applied. Frame-0 geometry was compared with probes for the list wheel, sheet and nested view. Matching configurations: same physics, same drag path and timing, same velocity input (move cadence), same sizes at frame 0.

### Analysis of the new twins

| Twin | Result | Attribution |
| --- | --- | --- |
| `scroll__drag__notification_order` | Final offset Flutter 300, Incular 240; updates 5 vs 4; end and start match; user-scroll 2 vs 0 | First move (60 px) lost in Incular at 128 ms: the same drop as gestures-001 (singleton arena). Remaining steps match one move later, value for value. The user-scroll gap is open question Q1. |
| `scroll__fling__notification_order` | Final offset 1400 both; overscroll 1 both; update 15 vs 16; user-scroll 2 vs 0 | Final state matches. The first-move drop changes the path but not the clamped end. User-scroll gap as Q1. |
| `scroll__drag_overscroll__clamping` | Final offset 0 both; overscroll 4 vs 3 | Exactly one overscroll per lost 50 px move; gestures-001. |
| `scroll__drag_overscroll__bouncing` | Overscroll 0 (Flutter) vs 32 (Incular); final offset -6.35 vs -5.88 | Not a twin mismatch: Flutter BouncingScrollPhysics has no parent, so applyBoundaryConditions returns 0 (scroll_physics.dart:308-309) and setPixels never reports overscroll (scroll_position.dart:365-396). Recorded as scroll-011. |
| `scroll__page_view__fling_page_changed` | Frame 3 offset 90 px behind (first move lost); final page 1, page_changes 1, last_page 1 on both | Transient gestures-001 effect only. Final match. Also shows the listener workaround for scroll-007. |
| `scroll__list_wheel__selected_fling` | Frame 0 selected_changes 1 vs 0; item0 x 0-300 vs 250-550; final offset 0 vs 1950 | Layout difference (scroll-010) puts the child at the left edge, so the drag at x=400 misses it. Callback at first layout is scroll-009. |
| `scroll__draggable_sheet__drag_extent` | Frame 0 geometry matches; final size 0.5 vs 1.0; list does not move at all | Not explained by the first move. Recorded as scroll-013 (medium confidence). |
| `scroll__nested_scroll_view__drag_offsets` | Frame 0 header and item0 match; final outer 0 vs 200, inner 0 vs 1232 | Not explained by the first move. Recorded as scroll-012 (medium confidence). |
| `scroll__single_child__keep_offset_page_storage` | Match | none |

### Findings

- scroll-001 to scroll-005: accepted, revalidated at a19bf487 (see the sections above).
- scroll-006: rejected. Duplicate of focus_input-003 (PageDown does not scroll a focused list). Cross-reference: `audits/flutter-parity/findings/focus_input.md`.
- scroll-007: downgraded to P2 (missing callback surface). The listener workaround exists: a controller notification listener on `PageController` (the `ScrollController` alias, `crates/incular-widgets/src/scrolling/scroll_views.rs:1036`) reports page changes, and the scenario `scroll__page_view__fling_page_changed` reaches the same final count and page as Flutter `onPageChanged`.
- scroll-008: unchanged (pending review, image package).
- scroll-009 (P2): ListWheel reports the initial item through the selection callback; Flutter seeds `_lastReportedItemIndex` (list_wheel_scroll_view.dart:748-775), Incular fires at first layout (wheel.rs:884-895).
- scroll-010 (P2): ListWheel children are left-aligned, not centred (frame 0 probe: x 0 to 300 vs 250 to 550).
- scroll-011 (P2): Bouncing drag emits OverscrollNotification in Incular (32) and none in Flutter (0). Medium confidence on the exact Incular path.
- scroll-012 (P1, medium): NestedScrollView does not respond to a touch drag in the harness (outer and inner stay 0 while Flutter reaches 200 and 1232). Frame 0 geometry matches.
- scroll-013 (P1, medium): DraggableScrollableSheet does not expand under a drag from its list (size stays 0.5; Flutter reaches 1.0). Frame 0 geometry matches.

Both P1 drag findings are "pending confirmation": the same drag path scrolls a plain ListView in the same harness, so the gap is in gesture ownership for these widgets, not in the input path. They should be confirmed in an application before acceptance; if confirmed as wrong final state in common use they become P0.

### Deduplication

- gestures-001 (first pointer move dropped under the singleton arena) explains every first-step difference in the drag, fling, overscroll, PageView and sheet twins. It is not re-reported here.
- physics and slivers findings: no overlap with scroll-009 to scroll-013. The physics bouncing and clamping twins match.
- focus_input-003: covers the keyboard gap that scroll-006 proposed.

### Open questions

- Q1: Incular's controller-level listener counted zero UserScroll notifications under touch drags, while Flutter counts two per drag. `notify_user_scroll` is called from `crates/incular-widgets/src/tree/interaction.rs:3195`, so the drag path may not reach it. Not recorded as a finding until the call path is traced.
- Q2: Flutter's ScrollStart notification fires at the touch-down frame (before any move) in the drag twin. Not yet traced in Flutter source; not recorded.
- Q3: ListWheel looping delegate (`ListWheelChildLoopingListDelegate`) and `OverscrollIndicatorNotification` have no Incular counterpart by name (rg), and `ScrollDirection` has none either. Observable; no finding raised in this pass.
- Q4: `ScrollHoldController`, `HoldScrollActivity`, `IdleScrollActivity` and `ScrollActivityDelegate` have no type counterparts; the hold behaviour is inside `crates/incular-scroll/src/activity.rs` and is exercised by scroll-012 and scroll-013.
- Q5: Multi-position `ScrollController` behaviour (`TrackingScrollController`) is out of scope by Incular's single-attachment contract.

### Harness requests

- RawScrollbar thumb geometry and fade opacity: not visible to the frame recorder. Flutter paints the thumb through `ScrollbarPainter` inside a custom paint (not a probe-able widget) and fades it through painter state, not a `RenderOpacity` layer, so neither `h.probe` nor the opacity field sees them. Requested: a pixel scene with a fixed clock that captures the thumb at t = 0, 100, 200, 400 ms after the last scroll, on both sides, plus `h.value` reading `ScrollbarPainter` fade progress if the recorder can expose it.
- No other request. Drag, fling, wheel and keyboard inputs are already available through the existing harness methods.

### Coverage

- All 104 rows of `coverage/scroll.jsonl` have a final status; no row keeps "clean stop before audit", and no counterpart is UNRESOLVED (names searched with `rg -n`; not-found names are `MISSING`). Rows owned by slivers carry `"reassign":"slivers"` with status `out-of-scope`.
- Counts after this pass: partial-reading 48, out-of-scope 29, missing 15, partial-scenario 12, unchecked 0 (104 rows).

### Validation run (final)

- `harness/run-frames.sh scroll__` at a19bf487: 7 of 17 match; divergences as above.
- `cargo check -q -p incular-parity-harness --tests`: passed.
- `flutter analyze test/frames/scroll_test.dart`: no issues.
- `render-inventory.ps1`: exit 0.
- JSONL: `findings/scroll.jsonl` (13 lines) and `coverage/scroll.jsonl` (104 lines) parse.

### Finding sections (resumed pass)

### scroll-006 — Rejected: duplicate of focus_input-003

- Status: rejected (duplicate). Cross-reference: focus_input-003 "PageDown does not scroll a focused list".
- Reason: the same keyboard gap is already recorded and scenario-backed under focus_input. The mechanism is the same (no ScrollIntent route for scroll views). Flutter binding evidence (`widgets/app.dart:1283-1330`) should be added to focus_input-003 rather than kept as a separate finding.

### scroll-007 — PageView has no onPageChanged callback (P2 after downgrade)

- Severity: P2 (downgraded from P1). Category: behavior. Impact class: missing-feature.
- Flutter: `widgets/page_view.dart:844, 940-958`. Incular: `crates/incular-widgets/src/scrolling/scroll_views.rs:1056` (no callback field).
- Evidence: scenario `scroll__page_view__fling_page_changed`. A controller-level notification listener reports page changes (rounded pixels/800), and the final count and page equal Flutter's (1 and 1). This is the listener workaround that justifies P2.
- Impact: callers must attach a controller listener and do the page arithmetic themselves; no PageView-level callback.
- Fix direction: an `on_page_changed` callback on PageView that wraps the same depth-0 update logic.
- Confidence: high. Limitation: first-callback timing not compared beyond the final count.

### scroll-009 — ListWheel reports the initial selected item (P2)

- Flutter: `widgets/list_wheel_scroll_view.dart:748-775`. Incular: `crates/incular-widgets/src/advanced_scrolling/wheel.rs:884-895`.
- Evidence: scenario `scroll__list_wheel__selected_fling`, frame 0: selected_changes Flutter 0, Incular 1; selected_index 0 on both.
- Impact: a spurious selection callback at mount.
- Cause: `report_selection` starts with no last reported index, so the first layout reports index 0 as a change.
- Fix direction: seed the last reported index from the initial item, as Flutter's `_lastReportedItemIndex` does.
- Confidence: high.

### scroll-010 — ListWheel children are left-aligned, not centred (P2)

- Flutter: `widgets/list_wheel_scroll_view.dart`, `rendering/list_wheel_viewport.dart`. Incular: `crates/incular-widgets/src/advanced_scrolling/wheel.rs:457`.
- Evidence: scenario `scroll__list_wheel__selected_fling`, frame 0 probe `item0`: Flutter x 250 to 550, Incular x 0 to 300 (same vertical position).
- Impact: narrow wheel items sit at the left edge and only respond to touches there; the scenario drag at x=400 never reaches an item, so the list does not scroll (final offset 0 against Flutter 1950).
- Cause: no horizontal centring of children on the cross axis in the wheel viewport.
- Fix direction: centre children on the cross axis and keep hit-testing aligned with the painted position.
- Confidence: high for geometry; the scroll-failure link is inferred from the hit point and not isolated.

### scroll-011 — Bouncing drag past the edge emits OverscrollNotification (P2)

- Flutter: `widgets/scroll_physics.dart:308-309` (default applyBoundaryConditions returns the parent's value or 0; no parent here) and `widgets/scroll_position.dart:365-396` (didOverscrollBy only when setPixels gets a nonzero boundary overscroll). Incular: `crates/incular-scroll/src/activity.rs:328`.
- Evidence: scenario `scroll__drag_overscroll__bouncing`: overscroll notifications Flutter 0, Incular 32; final offsets -6.35 and -5.88 (close, as expected for the lost first move).
- Impact: listeners that treat overscroll as a boundary signal fire for bounce only in Incular.
- Cause: Incular's bouncing path reports Overscroll from its boundary handling.
- Fix direction: emit Overscroll only for clamped amounts (what Flutter's applyBoundaryConditions returns); bouncing emits none.
- Confidence: medium. Limitation: one drag; the exact Incular branch was not traced line by line.

### scroll-012 — NestedScrollView does not scroll under a touch drag (P1, pending confirmation)

- Flutter: `widgets/nested_scroll_view.dart:183, 1990`. Incular: `crates/incular-widgets/src/scrolling/sliver_descriptors.rs:596` (NestedScrollView converts to CustomScrollView with the body as a SliverToBoxAdapter, lines 630-640).
- Evidence: scenario `scroll__nested_scroll_view__drag_offsets`. Frame 0: header 0 to 200 and item0 at 200 on both sides. Frame 3: Flutter header at -50 (outer 50), Incular unchanged. Final: Flutter outer 200 and inner 1232; Incular 0 and 0. A plain ListView drag in the same harness scrolls on Incular.
- Impact: touch drags over a nested body do nothing in Incular.
- Cause: not isolated. Candidates: gesture ownership between the outer viewport and a body that is not an inner scrollable, or hit-testing through the adapter.
- Fix direction: confirm in an application; route drags to the inner position first and coordinate the outer position as Flutter's NestedScrollViewState does.
- Confidence: medium. Severity P1 pending confirmation (P0 if it is a wrong final state in common use when confirmed).

### scroll-013 — DraggableScrollableSheet does not expand under a drag from its list (P1, pending confirmation)

- Flutter: `widgets/draggable_scrollable_sheet.dart`. Incular: `crates/incular-widgets/src/advanced_scrolling/draggable.rs:748-760`.
- Evidence: scenario `scroll__draggable_sheet__drag_extent`. Frame 0: sheet top 300 and item0 at 300 on both sides. Final: Flutter size 1.0 after an upward 200 px drag; Incular 0.5 with the list unmoved.
- Impact: sheets built on a scrollable list stay at their initial size under touch.
- Cause: not isolated; candidates are gesture ownership for the inner list or the sheet not receiving upward drags from its controller.
- Fix direction: confirm in an application; pass upward drags to the sheet until max extent, as Flutter does.
- Confidence: medium. Severity P1 pending confirmation.

### Revised queue

- scroll-006: rejected (duplicate of focus_input-003).
- scroll-007: P2 (listener workaround exists).
- scroll-009 to scroll-013: pending-review in `findings/scroll.jsonl`.
