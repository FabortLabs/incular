# slivers: Viewport and sliver widgets

## Summary

Nine deterministic frame twins cover static sliver layout, pinned headers, cache extent, and a floating-header scroll transition. Seven scenarios match; two divergences are recorded: varied-extent children lose their builder-provided size in Incular (`slivers-001`), and Incular's default floating header omits Flutter's scroll-end snap animation (`slivers-002`). The explicit 180 px cache-extent twin builds the same `cache0` through `cache19` probes on both sides in all three frames. Flutter's slivers test file passes all nine tests; the filtered Incular frame run confirms the same seven matches and two divergences.

## Coverage

| Flutter item | Incular counterpart | Method | Scenarios | Result |
| --- | --- | --- | --- | --- |
| `CustomScrollView`, vertical forward axis | `CustomScrollView` | scenario | all `slivers__*` | Static geometry and the exercised scrolling scenes are covered. Horizontal/reverse axes, centered viewports, paint order, and semantic counts remain unchecked. |
| `SliverToBoxAdapter` | `SliverToBoxAdapter` | scenario | `slivers__box_padding_fill_remaining__static_and_scroll` | Match. |
| `SliverPadding` | `SliverPadding` | scenario | `slivers__box_padding_fill_remaining__static_and_scroll` | Match for symmetric physical insets; directional padding and overlap are unchecked. |
| `SliverList` | `SliverList` | scenario | `slivers__box_padding_fill_remaining__static_and_scroll` | Three explicit children match. Lazy builder lifecycle, keep-alive, repaint-boundary, and semantic-index options are unchecked. |
| `SliverFixedExtentList` | `SliverFixedExtentList` | scenario | `slivers__fixed_prototype_extent_lists__indices`, `slivers__pinned_header_and_main_axis_group__scroll`, `slivers__custom_scroll_view__cache_extent` | Match for fixed extents, including the exercised explicit cache window. |
| `SliverPrototypeExtentList` | `SliverPrototypeExtentList` | scenario | `slivers__fixed_prototype_extent_lists__indices` | Match with a 44 px prototype. Prototype changes and lazy lifecycle are unchecked. |
| `SliverVariedExtentList` | `SliverVariedExtentList` | scenario + source | `slivers__varied_extent_list__indices` | Divergent; see `slivers-001`. |
| `SliverGrid`, fixed cross-axis count | `SliverGrid`, `SliverGridDelegate` | scenario | `slivers__grid_and_fill_viewport__geometry` | Match with three columns, 54 px main extent, 6 px main spacing, and 8 px cross spacing. Max-cross-axis delegate, RTL, and reverse-cross-axis cases are unchecked. |
| `SliverFillRemaining(hasScrollBody: false)` | `SliverFillRemaining` | scenario | `slivers__box_padding_fill_remaining__static_and_scroll` | Match at initial layout and through the tested drag. Scroll-body true and overscroll modes are unchecked. |
| `SliverFillViewport(viewportFraction, padEnds)` | `SliverFillViewport` | scenario | `slivers__grid_and_fill_viewport__geometry` | Match at viewport fraction 0.75 with `padEnds: false`. Other fractions and end padding are unchecked. |
| `SliverPersistentHeader(pinned: true)` | `SliverPersistentHeader` | scenario + source | `slivers__pinned_header_and_main_axis_group__scroll` | Match for a fixed 70 px pinned header after `jumpTo(135)`. Variable min/max extents and overlap/stretch are unchecked. |
| `SliverMainAxisGroup` | `SliverMainAxisGroup` | scenario | `slivers__pinned_header_and_main_axis_group__scroll` | Match in the fixed-extent pinned-header scene. Trailing-edge group behavior is unchecked. |
| `SliverCrossAxisGroup` | `SliverCrossAxisGroup` | scenario | `slivers__cross_axis_group__lanes` | Match for two equal-width lanes. Expanded flex and unequal extents are unchecked. |
| `PinnedHeaderSliver` | `PinnedHeaderSliver` | scenario + source | `slivers__pinned_header_sliver__child_extent` | Match for a 60 px child at offset 80. Horizontal axis, empty child, and overlap are unchecked. |
| `CustomScrollView.cacheExtent: 180` | `.cache_extent(180)` | scenario + source | `slivers__custom_scroll_view__cache_extent` | Match: both sides materialize `cache0` through `cache19` across all three frames. Other cache extents, scroll positions, and shrink-wrap caching are unchecked. |
| `SliverFloatingHeader` default snap | `SliverFloatingHeader` default `snap: false` | scenario + source | `slivers__floating_header__default_snap_after_wheel_scroll` | Divergent; see `slivers-002`. `snapMode.scroll`, custom animation styles, touch drags, and interrupted snaps are unchecked. |
| `SliverResizingHeader`, overscroll behavior | `SliverResizingHeader` | reading only | Incular `resizing_headers.rs` | Flutter extent interpolation, scroll transitions, overscroll, and snapping are not compared. |
| `ShrinkWrappingViewport` | `ShrinkWrappingViewport` | not checked | — | Its separate shrink-wrap measurement and offset setup were outside the runnable twins; no parity claim is made. |
| Other widget slivers, including animated lists/grids, visibility, safe-area, opacity, and overlap wrappers | Corresponding or lower-level Incular slivers | not checked | — | Their widget-level behavior and mapping remain to be classified. Rendering protocol helpers and two-dimensional viewport declarations are also outside these twins. |

### Source comparison notes

- Flutter's `CustomScrollView` passes `cacheExtent` through the scroll view to the viewport (`widgets/scroll_view.dart:356-357,479`); `RenderViewport` gives each sliver the viewport extent plus the cache window on both sides and a negative cache origin (`rendering/viewport.dart:2020-2021`). Incular stores the extent on `CustomScrollView` and passes it to the viewport (`crates/incular-widgets/src/scrolling/sliver_descriptors.rs:397-398,503-505,548`); its sequence forwards and consumes per-sliver cache geometry (`crates/incular-widgets/src/scrolling/layout.rs:1889-1915,1957-1959`). The explicit 180 px twin matches with 20 children materialized on both sides.
- Flutter's `PinnedHeaderSliver` lays out its child with sliver constraints, uses the measured child extent for scroll extent, and clamps layout extent against scroll offset (`widgets/pinned_header_sliver.dart:67-105`). Incular's descriptor maps it to a pinned box sliver (`crates/incular-widgets/src/scrolling/sliver_descriptors.rs:1772-1800`; `scrolling/layout.rs:74-85`). The 60 px child at offset 80 matches in the paired trace.
- Flutter's pinned persistent-header render object derives `layoutExtent` from `maxExtent - scrollOffset` and reports paint/obstruction geometry (`rendering/sliver_persistent_header.dart:404-442`). The fixed-extent pinned-header twin matches under a `SliverMainAxisGroup`; delegate resizing and floating combinations remain open.

## Findings

### slivers-001 — Varied extent children are measured at intrinsic size

- Severity: P1
- Category: layout
- Flutter: `rendering/sliver_fixed_extent_list.dart:268-270` reads the item extent from `itemExtentBuilder` and supplies it as both the minimum and maximum child main-axis extent. `widgets/sliver.dart:597-677` wires `SliverVariedExtentList` to that render object.
- Incular: `crates/incular-widgets/src/scrolling/sliver_descriptors.rs:891-900` initializes `MeasuredExtentIndex` with the per-index values. But `crates/incular-widgets/src/scrolling/layout.rs:397-404` builds each varied child's constraints using `sliver_child_constraints(..., None)`, leaving the main axis unconstrained; the post-layout path records the measured size at `:415-430`.
- Evidence: scenario `slivers__varied_extent_list__indices`, with the same four extents `[30, 37, 44, 51]` and dimensionless solid-color children. At frame 0, Flutter reports item 3 at rect `[0,111,800,162]`, size `[800,51]`; Incular reports `[0,0,800,0]`, size `[800,0]`. Items 0, 1, and 2 are present in Flutter but absent in Incular. This remains through all three recorded frames in `harness/out/frames/report-slivers__.md`.
- Impact: a `SliverVariedExtentList` whose children rely on `itemExtentBuilder` for main-axis size can collapse to zero and disappear, shifting following slivers and corrupting scroll extent.
- Likely cause: Incular uses callback extents as index estimates but does not apply them as child layout constraints before recording measured size.
- Fix direction: pass each materialized item's callback extent as a tight main-axis constraint, matching Flutter's fixed-extent adaptor behavior.
- Confidence: high.

### slivers-002 — Floating headers omit Flutter's default scroll-end snap

- Severity: P0
- Category: layout
- Flutter: `widgets/sliver_floating_header.dart:137-142` listens to `ScrollPosition.isScrollingNotifier`; `:209-249` starts a snap whenever scrolling stops with the header partly visible, using a default 300 ms ease-in-out animation. `:76-81` says `snapMode` defaults to `overlay`; `:304-312` uses that mode to choose layout extent, so it does not disable the snap.
- Incular: `crates/incular-widgets/src/scrolling/sliver_descriptors.rs:1828-1851` defaults `SliverFloatingHeader.snap` to `false`; `:1882` subscribes to scroll activity only when that flag is enabled. The snap runner also requires `spec.enabled` (`crates/incular-widgets/src/scrolling/layout.rs:577-595`), with shared 300 ms timing at `:638`.
- Evidence: `slivers__floating_header__default_snap_after_wheel_scroll` uses a 70 px header, 18 fixed 40 px rows, `jumpTo(100)`, then one -40 px wheel sample. Both controllers end at offset 60. At frame 5 (80 ms), Flutter's header rect is `[0,-60,800,10]`; Incular's is `[0,-30,800,40]`. Flutter animates the header fully into view by frame 24 (384 ms), rect `[0,0,800,70]`, while Incular remains at `[0,-30,800,40]` through the final frame. The viewport/body probes and offset agree; this isolates the header presentation difference.
- Impact: callers using Incular's floating-header default can leave a partly visible header stranded after a scroll activity ends, while Flutter settles it to an edge.
- Likely cause: Incular exposes snap as opt-in although Flutter's `SliverFloatingHeader` always performs the partial-header end animation; Flutter's `snapMode` chooses overlay versus scrolling layout, not whether snapping occurs.
- Fix direction: match Flutter's default end snap and make snap presentation modes independent of the enable/disable switch; retain an explicit opt-out only if required by Incular's API contract.
- Confidence: high.

## Pixel scene proposals

None. The confirmed issues are visible in frame geometry; this pass did not run pixel comparisons.

## Harness requests

None for this package.

## Open questions

Floating-header `snapMode.scroll`, custom/reverse animation styles, interrupted snaps, and touch-drag settling still need dedicated twins. Also unchecked are resizing-header transitions, `ShrinkWrappingViewport`, cache origins near content bounds, builder child lifecycle and semantic options, reverse/horizontal axes, and the remaining widget slivers listed above. The package inventory's lower-level rendering declarations need classification in the full audit.
