# slivers: Viewport and sliver widgets

## Summary

Six deterministic frame twins were added for box adapters, padding, fill slivers, fixed and prototype extent lists, varied extent lists, grids, fill viewport, pinned headers, main and cross axis groups, and cache extent. The first five validated groups produce one confirmed divergence: Incular's `SliverVariedExtentList` does not constrain children to the extent returned by its extent builder. That allows zero-size children to overwrite the measured extent index and removes their scroll extent. Five other scenarios match. The latest eight-scenario Flutter file passed, but the Incular comparison is blocked by a compile error in a concurrently edited transitions scenario file.

## Coverage

| Flutter item | Incular counterpart | Method | Scenarios | Result |
| --- | --- | --- | --- | --- |
| `CustomScrollView` with vertical axis, forward direction | `CustomScrollView` | scenario | all `slivers__*` | Static geometry and the matched scrolling scene agree for exercised combinations. Axis, reverse, paint order and semantic counts remain unchecked. |
| `SliverToBoxAdapter` | `SliverToBoxAdapter` | scenario | `slivers__box_padding_fill_remaining__static_and_scroll` | Match. |
| `SliverPadding` | `SliverPadding` | scenario | `slivers__box_padding_fill_remaining__static_and_scroll` | Match for symmetric physical insets; directional padding is unchecked. |
| `SliverList` | `SliverList` | scenario | `slivers__box_padding_fill_remaining__static_and_scroll` | Three explicit children match. Builder virtualization, keep-alive and semantic-index options are unchecked. |
| `SliverFixedExtentList` | `SliverFixedExtentList` | scenario | `slivers__fixed_prototype_extent_lists__indices`, `slivers__pinned_header_and_main_axis_group__scroll`, `slivers__custom_scroll_view__cache_extent` | Match for fixed extents. Cache scenario has Flutter evidence; Rust comparison awaits the shared compile fix. |
| `SliverPrototypeExtentList` | `SliverPrototypeExtentList` | scenario | `slivers__fixed_prototype_extent_lists__indices` | Match with a 44 px prototype. |
| `SliverVariedExtentList` | `SliverVariedExtentList` | scenario + reading | `slivers__varied_extent_list__indices` | Divergent: builder extents are not applied as child constraints in Incular. |
| `SliverGrid` with fixed cross-axis count | `SliverGrid` and `SliverGridDelegate` | scenario | `slivers__grid_and_fill_viewport__geometry` | Match with 3 columns, 54 px main extent, 6 px main spacing and 8 px cross spacing. Max-cross-axis delegate and RTL/reversed cross-axis cases are unchecked. |
| `SliverFillRemaining(hasScrollBody: false)` | `SliverFillRemaining` | scenario | `slivers__box_padding_fill_remaining__static_and_scroll` | Match at initial layout and through the tested drag. Scroll-body true and overscroll modes are unchecked. |
| `SliverFillViewport(viewportFraction, padEnds)` | `SliverFillViewport` | scenario | `slivers__grid_and_fill_viewport__geometry` | Match at viewport fraction 0.75 with `padEnds: false`. Other fractions and end padding are unchecked. |
| `SliverPersistentHeader(pinned: true)` | `SliverPersistentHeader` | scenario | `slivers__pinned_header_and_main_axis_group__scroll` | Match for a fixed 70 px header under a direct `jumpTo(135)` scroll. Floating, snap and variable min/max extents are unchecked. |
| `SliverMainAxisGroup` | `SliverMainAxisGroup` | scenario | `slivers__pinned_header_and_main_axis_group__scroll` | Match in the fixed-extent pinned-header scene. |
| `SliverCrossAxisGroup` | `SliverCrossAxisGroup` | scenario | `slivers__cross_axis_group__lanes` | Match for two equal-width lanes. Expanded flex and unequal extents are unchecked. |
| `PinnedHeaderSliver` | `PinnedHeaderSliver` | scenario drafted | `slivers__pinned_header_sliver__child_extent` | Flutter twin passes; Incular twin awaits a clear Cargo compile. |
| `CustomScrollView.cacheExtent` | `.cache_extent` | scenario drafted | `slivers__custom_scroll_view__cache_extent` | Flutter twin passes; Incular twin awaits a clear Cargo compile. |
| `ShrinkWrappingViewport` | `ShrinkWrappingViewport` | not checked | — | Its required `ViewportOffset` setup and shrink-wrap measurement were outside this pass's runnable cases; no parity claim is made. |
| `SliverFloatingHeader`, snap modes | `SliverFloatingHeader` | reading only | Incular `floating_headers.rs` and `snap_headers.rs` | Detailed Flutter motion and snap timing are unchecked; Incular tests are not Flutter twins. |
| `SliverResizingHeader` and overscroll behavior | `SliverResizingHeader` | reading only | Incular `resizing_headers.rs` | Flutter extent interpolation, scroll behavior, overscroll and snapping were not compared. |
| Render-sliver protocol helpers and two-dimensional viewport declarations in the preliminary inventory | lower-level renderer and advanced-scrolling APIs | not checked | — | Not widget-level sliver behavior in these twins; their relevance and mapping remain to be classified in the full inventory. |

## Findings

### slivers-001 — Varied extent children are measured at intrinsic size

- Severity: P1
- Category: layout
- Flutter: `rendering/sliver_fixed_extent_list.dart:268-270` reads the item extent from `itemExtentBuilder` and supplies it as both the minimum and maximum child main-axis extent. `widgets/sliver.dart:597-677` wires `SliverVariedExtentList` to that render object.
- Incular: `crates/incular-widgets/src/scrolling/sliver_descriptors.rs:891-900` initializes `MeasuredExtentIndex` with the per-index values. But `crates/incular-widgets/src/scrolling/layout.rs:397-404` builds each varied child's constraints using `sliver_child_constraints(..., None)`, leaving the main axis unconstrained; the post-layout path records the measured size at `:415-430`.
- Evidence: scenario `slivers__varied_extent_list__indices`, with the same four extents `[30, 37, 44, 51]` and dimensionless solid-color children. At frame 0, Flutter reports varied item 3 at rect `[0,111,800,162]`, size `[800,51]`. Incular reports it at `[0,0,800,0]`, size `[800,0]`; items 0, 1 and 2 are present in Flutter but absent in Incular. The difference persists for all three recorded frames. The same Flutter scenario was rerun with `flutter test test/frames/slivers_test.dart`; the Rust comparison is captured in `harness/out/frames/report-slivers__.md` from the last complete filtered run.
- Impact: a `SliverVariedExtentList` whose children rely on `itemExtentBuilder` for their main-axis size can collapse to zero and disappear, shifting following slivers upward and corrupting scroll extent.
- Likely cause: Incular uses callback extents as index estimates but does not apply them to child layout constraints before recording measured child size.
- Fix direction: pass the index's extent as a tight main-axis child constraint for each materialized varied-extent item, matching Flutter's fixed-extent adaptor behavior.
- Confidence: high

## Pixel scene proposals

None. The confirmed issue is layout geometry captured directly by the frame harness; no paint-only sliver divergence was observed here.

## Harness requests

None for this package.

## Open questions

The Flutter sliver widgets not listed with a completed twin above need review, especially floating/resizing header transitions, shrink wrapping, cross-axis flex, sliver visibility/semantics, and lazy child delegate lifecycle. The package inventory also contains 61 rendering declarations and multiple helper types whose widget-level relevance needs the main audit to classify.
