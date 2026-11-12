# scroll: Scroll positions, views, and scrolling widgets

## Summary

Three frame twins compare controller jumps, controller animation and retargeting, and a fractional PageView with a nonzero initial page. Normal jumps, animation retargeting, and fractional page layout match. An out-of-range `ScrollController.jumpTo` differs: Flutter exposes the requested offset and springs back, while Incular clamps the jump immediately. Source review also found PageController conveniences, configurable initial scroll offset, and `Scrollable.ensureVisible` are missing from the corresponding Incular APIs. Existing tests cover many scroll physics, page layout, scrollbar geometry, wheel behavior, overscroll effects, and draggable sheets, but this package audit did not run those suites or exhaust every inventory entry.

## Coverage

| Flutter item | Method / evidence | Result |
| --- | --- | --- |
| `ScrollController.jumpTo` and metrics | `scroll__controller__jump_clamp`; frame 0 and in-range jump match; out-of-range jump differs at frame 3. | divergent |
| `ScrollController.animateTo` and retargeting | `scroll__controller__animate_retarget`: 320 ms fast-out-slow-in animation retargeted to 250 px over 160 ms linear; values match. Existing `crates/incular-widgets/tests/scroll_animate_to.rs` covers curve progress, jump cancellation, replacement, edge stopping, zero duration and reduced motion. | match for the sampled path |
| `ScrollController.initialScrollOffset` | Flutter `scroll_controller.dart:74-95`; Incular `ScrollController::new` has no initial-offset argument (`crates/incular-scroll/src/controller.rs:100-104`). | missing API |
| `ScrollPosition` and ballistic activities | Source review of `scroll_position.dart`, `scroll_position_with_single_context.dart`, Incular activity/controller implementation; existing `crates/incular-scroll/tests/activity.rs` and `attachment.rs`. Flutter’s public position/attach lifecycle is represented by Incular’s metric attachment/activity APIs rather than a public `ScrollPosition` object. | partial; lifecycle comparison not frame-tested |
| `ScrollPhysics` (`Clamping`, `Bouncing`, `NeverScrollable`, `AlwaysScrollable`, `RangeMaintaining`, page/fixed-extent) | `crates/incular-scroll/tests/flutter_ballistics.rs`, `scroll.rs`, and `physics_audit.rs` contain Flutter-number and policy tests; source comparison of `widgets/scroll_physics.dart` and `crates/incular-scroll/src/physics.rs`. | partial; see the physics package findings for simulation divergences |
| `ScrollNotification` ordering | Source review of `widgets/scroll_notification.dart` and `incular-scroll/src/notifications.rs`; existing `crates/incular-scroll/tests/scroll.rs` checks activity order and reentrant callbacks. | partial; no paired frame input notification scenario |
| `PageView` and `PageController` | `scroll__page_view__fraction_initial_page` matches page index and geometry. Existing `crates/incular-widgets/tests/scroll_pages_reorder.rs` checks viewport fraction, page snapping, reverse, resize and identity. | partial; PageController API gaps in scroll-002 |
| `ListWheelScrollView` | Source review; existing `crates/incular-widgets/tests/wheel_scrolling.rs` and `wheel_axis_routing.rs` check projection, selection and axis handling. | partial; no frame twin |
| `NestedScrollView` | Source reviewed at `widgets/nested_scroll_view.dart`; Incular has a descriptor at `crates/incular-widgets/src/scrolling/sliver_descriptors.rs:596`. `crates/incular-scroll/tests/scroll.rs` checks nested delta coordination, but not the full widget lifecycle. | partial; no frame twin |
| `GlowingOverscrollIndicator` / `StretchingOverscrollIndicator` | Source review; existing `crates/incular-widgets/tests/glowing_overscroll.rs` and `stretch_overscroll.rs`. | partial; paint not checked with pixels |
| `RawScrollbar` | Source review of fade/interaction path and `crates/incular-widgets/src/advanced_scrolling/raw_scrollbar.rs`; existing `crates/incular-widgets/tests/advanced_scrolling.rs` checks thumb mapping and drag, and `scrollbar_pointer_kind.rs` checks pointer kind. | partial; no frame/pixel comparison |
| `DraggableScrollableSheet` | Source review; existing `crates/incular-widgets/tests/draggable_sheet.rs` and `advanced_scrolling_retained_integration.rs` exercise the sheet and controller. | partial; no frame twin |
| `Scrollable.ensureVisible` | Flutter static API exists at `widgets/scrollable.dart:491`; no corresponding helper was found under the Incular scroll/widget scroll modules. | missing API |
| Other scroll inventory declarations | The generated inventory has 104 rows assigned to `scroll`, including internal rendering helpers whose widget-level relevance is not reconciled. | not checked: remaining declarations need item-by-item review |

## Findings

### scroll-001 — Out-of-range jump is clamped instead of springing back

- Severity: P2
- Category: behavior
- Flutter: `widgets/scroll_position_with_single_context.dart:197-207` calls `forcePixels(value)` without clamping and then starts ballistic correction with `goBallistic(0.0)`.
- Incular: `crates/incular-scroll/src/controller.rs:246-263` clamps the requested value to `[0, max_offset]` before updating the controller.
- Evidence: `scroll__controller__jump_clamp`, identical initial frame (offset `0`, max `1400`, same row geometry) and identical in-range jump to `125`. At frame 3 / 48 ms after both twins request `100000`, Flutter reports offset `100000`; Incular reports `1400`. Flutter then springs toward the extent (`97856.2011` at 64 ms and `92679.3046` at 80 ms). The filtered `scroll__` harness was rerun.
- Impact: a programmatic jump outside the current extent immediately lands at the edge in Incular, while Flutter briefly moves to the requested position and corrects using scroll physics. This is visible with bouncing physics and changes notifications and intermediate scroll position values.
- Likely cause: Incular treats `jump_to` as a bounded assignment and does not create a ballistic activity for an out-of-range target.
- Fix direction: preserve the requested jump position, then start the configured out-of-range ballistic correction as Flutter does.
- Confidence: high

### scroll-002 — PageController page-based navigation API is missing

- Severity: P2
- Category: missing-api
- Flutter: `widgets/page_view.dart:119-125,174-226` gives `PageController` an `initialPage`, exposes a `page` getter, and provides `animateToPage` / `jumpToPage`.
- Incular: `crates/incular-widgets/src/scrolling/scroll_views.rs:1035-1036` aliases `PageController` to `ScrollController`; the available constructor is `ScrollController::new()` (`crates/incular-scroll/src/controller.rs:100-104`) and page navigation must be expressed in pixel offsets.
- Evidence: source reading. The `scroll__page_view__fraction_initial_page` twin reaches page 1 by passing Flutter `initialPage: 1` and an Incular `deferred_jump_to(400)` workaround; there is no Incular initial-page/page-index API or page-based jump/animate method.
- Impact: page-oriented callers must compute pixel offsets themselves and cannot ask a PageController to initialize, report, or navigate by page number.
- Likely cause: Incular models the page controller as the underlying pixel scroll controller without a page-coordinate facade.
- Fix direction: provide page controller configuration and page-coordinate operations that convert using viewport fraction and current viewport dimensions.
- Confidence: high

### scroll-003 — ScrollController cannot configure an initial offset

- Severity: P2
- Category: missing-api
- Flutter: `widgets/scroll_controller.dart:74-95` accepts `initialScrollOffset` and uses it for a newly attached position when there is no saved offset.
- Incular: `crates/incular-scroll/src/controller.rs:100-104` only exposes `ScrollController::new()`; offsets can be changed after construction with `jump_to` or `deferred_jump_to`.
- Evidence: source reading. No Incular constructor or builder option accepts an initial offset.
- Impact: a caller cannot configure the initial position as part of controller construction; it must issue a separate command and handle the viewport attachment/layout timing.
- Likely cause: controller initialization only stores defaults and restoration state.
- Fix direction: add an initial-offset configuration and apply it when the first viewport publishes extents, with restoration taking Flutter-compatible precedence.
- Confidence: high

### scroll-004 — Scrollable.ensureVisible has no Incular equivalent

- Severity: P2
- Category: missing-api
- Flutter: `widgets/scrollable.dart:491-531` walks enclosing scrollables and requests aligned visibility for the target render object, with configurable duration, curve and alignment policy.
- Incular: no corresponding `ensure_visible` / `ensureVisible` API appears in `crates/incular-widgets/src/scrolling`, `crates/incular-widgets/src/advanced_scrolling`, or `crates/incular-scroll/src`.
- Evidence: source/API search and source reading of Flutter's helper.
- Impact: applications cannot request that a focused or selected descendant be brought into view through the widget-layer scrolling API, particularly across nested viewports.
- Likely cause: there is no public widget-tree operation that resolves a descendant's geometry through its enclosing scroll positions.
- Fix direction: add a tree-aware operation that walks ancestor scroll viewports and applies Flutter-compatible alignment and animation policies.
- Confidence: high

## Pixel scene proposals

No scene was added. `RawScrollbar` fade, thumb thickness, and overscroll indicators are paint-visible candidates for the orchestrator's pixel batch; the current pixel scene harness is static and cannot verify fade timing.

## Harness requests

None for the paired controller and page scenarios. Scroll notification order, nested viewport coordination, and two-pointer gesture interactions still need dedicated frame scenarios if they are not covered by other packages.

## Open questions

- Review the 104 inventory rows assigned to scroll and determine which rendering-level declarations affect the requested widget behavior.
- Run and compare `RawScrollbar`, wheel, nested viewport, draggable sheet, overscroll indicator and ensure-visible behaviors in a scene-level harness. Some APIs lack a currently suitable public way to inspect callbacks or retained geometry from the frame harness.
- Confirm whether Incular intends its explicit single-live-metric-attachment contract to diverge from Flutter ScrollController's multi-position attachment model; no duplicate finding is made here.
