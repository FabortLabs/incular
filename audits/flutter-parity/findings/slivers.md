# slivers: Viewport and sliver widgets

Revision: HEAD `a19bf487`. Flutter 3.41.7.

## Summary

Fifteen paired frame twins cover sliver layout, pinned and floating headers, resizing headers, cache extent, fill viewports, grids, cross-axis groups, visibility wrappers and keep-alive. Nine match. Six diverge, and each maps to one finding:

- `slivers-001` (accepted): varied-extent children collapse to zero.
- `slivers-002` (accepted): floating headers omit the default end snap.
- `slivers-003` (P1): `SliverFillRemaining(hasScrollBody: false)` keeps a stale maximum extent after content above it grows.
- `slivers-004` (P1): `KeepAlive` does not retain a scrolled-away child.
- `slivers-005` (P2): the resizing header's cache contribution builds one trailing row too few.
- `slivers-006` (P3): children whose edge exactly meets a cache edge are built on the opposite side.

Source-only findings: `slivers-007` (no `fillOverscroll`), `slivers-008` (no floating-header snap mode or configuration), `slivers-009` (no persistent-header delegate with shrinkOffset), `slivers-010` (no public Viewport or ShrinkWrappingViewport with anchor or center).

The previously suspected offstage-opacity divergence was a recorder gap; after the recorder change `slivers__opacity_offstage__visibility` matches.

## Scenarios

| Scenario | Result | Notes |
| --- | --- | --- |
| `slivers__box_padding_fill_remaining__static_and_scroll` | match | adapter, symmetric padding, fill remaining with drag |
| `slivers__varied_extent_list__indices` | DIVERGES | `slivers-001` |
| `slivers__fixed_prototype_extent_lists__indices` | match | fixed 38 px and 44 px prototype |
| `slivers__grid_and_fill_viewport__geometry` | match | 3-column grid, fill viewport 0.75 without padEnds |
| `slivers__pinned_header_and_main_axis_group__scroll` | match | fixed pinned header at offset 135 |
| `slivers__cross_axis_group__lanes` | match | two equal lanes |
| `slivers__pinned_header_sliver__child_extent` | match | 60 px child at offset 80 |
| `slivers__custom_scroll_view__cache_extent` | match | cacheExtent 180, rows 0-19 |
| `slivers__floating_header__default_snap_after_wheel_scroll` | DIVERGES | `slivers-002` |
| `slivers__resizing_header__collapse_scroll` | DIVERGES | `slivers-005`; header rect matches, rows 20-21 built only in Flutter at offset 60 |
| `slivers__fill_viewport__pad_ends_fraction` | match | viewport fraction 0.8 with padEnds |
| `slivers__list_builder__default_cache_window` | DIVERGES | `slivers-006`; boundary rows 14, 17, 37 |
| `slivers__opacity_offstage__visibility` | match | opacity 0.5, offstage child (after the recorder fix) |
| `slivers__fill_remaining_no_scroll_body__shrink_after_preceding_growth` | DIVERGES | `slivers-003`; final fill extent 555 px vs 400 px |
| `slivers__keep_alive__retains_scrolled_child` | DIVERGES | `slivers-004` (row 2 retained in Flutter only); rows 14, 17, 37 are the `slivers-006` boundary effect |

The keep-alive twin turns off the delegate's automatic keep-alives, semantic indexes and repaint boundaries on the Flutter side, so only the explicit `KeepAlive` is under test; the Incular side has no automatic equivalents.

## Coverage

All 104 rows of `coverage/slivers.jsonl` have a final status. Counts: scenario 11, scenario+reading 3, scenario+source 1, partial-scenario 26, partial-reading 4, out-of-scope 22, missing 7, unchecked 30. Every unchecked row names its reason (no twin in this pass, pixel scene needed, reassigned to semantics, or no counterpart after search). The Viewport row was corrected from `alert_dialog.rs` to MISSING after search. The UNRESOLVED counterparts were resolved by `rg` (missing where no counterpart exists).

Searches performed for missing rows: snap mode and snap configuration, persistent-header delegate, show-on-screen and reveal, paint order, tree-sliver state mixin, growth direction, two-dimensional child manager and parent data, and public Viewport and anchor.

## Findings

### slivers-001 — Varied extent children are measured at intrinsic size (accepted)
Severity P1, layout. Flutter `rendering/sliver_fixed_extent_list.dart:268-270`; Incular `crates/incular-widgets/src/scrolling/layout.rs:400-403`. Scenario `slivers__varied_extent_list__indices`, frame 0: Flutter item 3 at `[0,111,800,162]`, Incular `[0,0,800,0]`. Still reproduces at HEAD.

### slivers-002 — Floating headers omit Flutter's default scroll-end snap (accepted)
Severity P0, layout. Flutter `widgets/sliver_floating_header.dart:137-142,209-249`; Incular `crates/incular-widgets/src/scrolling/sliver_descriptors.rs:1831-1840`. Scenario `slivers__floating_header__default_snap_after_wheel_scroll`, frame 5: Flutter `[0,-60,800,10]`, Incular `[0,-30,800,40]`. Still reproduces at HEAD.

### slivers-003 — SliverFillRemaining without a scroll body keeps its largest past extent
Severity P1, layout, wrong-final-state. Flutter `rendering/sliver_fill.dart:154-166`; Incular `crates/incular-widgets/src/scrolling/layout.rs:1048-1051`. Scenario `slivers__fill_remaining_no_scroll_body__shrink_after_preceding_growth`: after the adapter above grows from 45 to 200 px, Flutter's fill settles at 400 px (rect ending 600) and Incular's stays at 555 px (rect ending 755), from frame 3 onward. Cause: `self.extent.get().max(remaining_extent)` carries the largest previous extent forward. Fix: recompute from the current remaining extent on every layout. Confidence high; only the growth direction was tested.

### slivers-004 — KeepAlive does not retain a scrolled-away child
Severity P1, behavior, missing-feature. Flutter `widgets/sliver.dart:1577-1600`, `rendering/sliver_multi_box_adaptor.dart:140-165`; Incular `crates/incular-widgets/src/advanced_slivers/keep_alive.rs:234-242`. Scenario `slivers__keep_alive__retains_scrolled_child`: at offset 1000 Flutter keeps row 2 (`present: true`) and Incular does not (frame 2). Incular's own doc describes the request as transparent because the retained tree has no parent-data channel. Fix: connect the request to the sliver child manager. Confidence high; one child and one offset tested.

### slivers-005 — Resizing header cache accounting builds one trailing row too few
Severity P2, layout, edge-case. Flutter `widgets/sliver_resizing_header.dart:223` uses `calculateCacheOffset(0..childExtent)`; Incular `crates/incular-widgets/src/scrolling/layout.rs:1224-1232` uses `pinned_geometry` (`layout.rs:31-45`), which Flutter's pinned persistent header matches (`rendering/sliver_persistent_header.dart:436`). At offset 60 the consumed cache is 60 in Flutter and 120 in Incular; rows 20 and 21 are built only in Flutter (scenario frame 3). Cause inferred from the formulas. Confidence medium.

### slivers-006 — Cache-edge boundary children are built on the opposite side
Severity P3, layout, edge-case. Scenario `slivers__list_builder__default_cache_window` (viewport 600, cache 250, rows 50 px). At offset 0 Flutter builds 0-16 and Incular 0-17 (row 17 starts exactly at the cache end); at offset 1000 Flutter builds 14-36 and Incular 15-37. Counts match; only the edge convention differs. A non-aligned 180 px cache on 40 px rows matches. Render path not traced. Confidence medium.

### slivers-007 — SliverFillRemaining has no fillOverscroll option
Severity P2, behavior, missing-feature. Flutter `widgets/sliver_fill.dart:264,291,298-300`, `rendering/sliver_fill.dart:215-245`; Incular `crates/incular-widgets/src/scrolling/sliver_descriptors.rs:969-990`, `layout.rs:1044-1056`. Incular has overscroll physics (`crates/incular-scroll/src/activity.rs:275-316`), so the option is reachable. Source-only; no twin (the harness does not script bouncing overscroll for slivers). Confidence high in the absence.

### slivers-008 — SliverFloatingHeader has no snapMode or snap configuration
Severity P2, behavior, missing-feature. Flutter `widgets/sliver_floating_header.dart:31,76-81,304-312`; Incular `crates/incular-widgets/src/scrolling/sliver_descriptors.rs:1828-1840` has only a snap bool and a fixed 300 ms timing. Source-only; `rg` for snap_mode, SnapMode and snap_config finds nothing. Confidence high.

### slivers-009 — SliverPersistentHeader has no delegate with variable extents
Severity P2, behavior, missing-feature. Flutter `widgets/sliver_persistent_header.dart:20-125`; Incular `crates/incular-widgets/src/scrolling/sliver_descriptors.rs:239-260` is fixed-height. `SliverResizingHeader` (`sliver_descriptors.rs:1935`) covers min and max extents but has no shrinkOffset-aware delegate build. Source-only. Confidence medium.

### slivers-010 — No public Viewport or ShrinkWrappingViewport with anchor or center
Severity P2, behavior, missing-feature. Flutter `widgets/viewport.dart:55,357`, `rendering/viewport.dart:1408`; Incular's viewport is the internal `single_sliver_viewport` (`crates/incular-widgets/src/scrolling/layout.rs:2469`). Source-only; no public Viewport or anchor found. Confidence medium.

## Pixel scene proposals

None in this pass. `slivers-006` and `decorated_sliver` decoration would need pixel comparisons.

## Harness requests

Done since the clean stop: the recorder now treats `RenderSliverOffstage` as hidden in `_opacity`, and non-finite numbers are recorded as strings on both sides. Still open:

- A bouncing-physics overscroll drag helper for slivers, needed to twin `slivers-007`.
- A tap/hit-test helper, needed for `SliverIgnorePointer` and the hit-test protocol rows.
- A paint-order probe or pixel scene for `SliverPaintOrder`.

## Open questions

Not checked in this pass (named in the coverage rows): SliverFillViewport fractions other than 0.75 and 0.8; SliverGrid delegates other than fixed count (including max cross-axis extent); reverse and horizontal axes (growth direction is a reverse flag in Incular); SliverLayoutBuilder rebuilds; SliverVisibility maintain flags; SliverConstrainedCrossAxis and SliverCrossAxisExpanded; SliverSafeArea; DecoratedSliver painting; tree slivers; two-dimensional viewport; ShrinkWrappingViewport; SliverSemantics and SliverEnsureSemantics (reassigned to semantics).

## Validation run

- `harness/run-frames.sh slivers__` at HEAD `a19bf487`: 9 of 15 match, as in the table above. The Flutter and Incular runs completed with no exceptions.
- `flutter analyze test/frames/slivers_test.dart`: no issues.
- `cargo check -q -p incular-parity-harness --tests`: 0 errors.
- Coverage and findings JSONL: every line parses; every finding ID referenced from a coverage row exists in `findings/slivers.jsonl`.
- Not run: `cargo fmt`/clippy on the workspace (not in scope for this audit worker); `rustfmt --edition 2024` was run on `harness/incular/tests/frames/slivers.rs`.

### slivers-011 — SliverVisibility has no replacement sliver or maintain flags

- Severity: P2. Category: missing-api (missing-feature). Added by the orchestrator from the coverage sweep; status accepted.
- Flutter: `widgets/visibility.dart:356-380 (SliverVisibility: replacementSliver, maintainState, maintainAnimation, maintainSize, maintainSemantics, maintainInteractivity)`. Incular: `crates/incular-widgets/src/scrolling/sliver_descriptors.rs:1725-1760`.
- Evidence (source): Incular SliverVisibility stores only visible and the sliver. Flutter can keep the hidden sliver's state, animation, size, semantics or interactivity, and can show a replacement sliver.
- Impact: Hidden slivers cannot keep their scroll extent (maintainSize) or state, so toggling visibility in a CustomScrollView shifts following slivers and discards state where Flutter can preserve them.
- Cause: The descriptor models only a boolean.
- Fix direction: Add the replacement sliver and maintain flags with Flutter's defaults and assertions.
- Confidence: high. Limitations: Source only (sweep candidate confirmed by the orchestrator).

### slivers-012 — Viewports have no paintOrder option

- Severity: P3. Category: missing-api (missing-feature). Added by the orchestrator from the coverage sweep; status accepted.
- Flutter: `rendering/viewport.dart:51 (SliverPaintOrder); widgets/viewport.dart:72,142 (Viewport.paintOrder)`. Incular: `MISSING: rg -n -i "paint_order|PaintOrder" crates/incular-widgets/src returns nothing`.
- Evidence (source): Flutter 3.41 lets a viewport paint its slivers with the first or last sliver on top; Incular has no equivalent option.
- Impact: Overlapping slivers (for example pinned headers over later content) cannot choose Flutter's alternative paint order.
- Cause: Not implemented.
- Fix direction: Add a paint-order option to the viewport and scroll views, defaulting to Flutter's firstIsTop.
- Confidence: high. Limitations: Source only; Incular's default order versus firstIsTop was not compared by pixels.
