# Switchers parity audit

Reference: Flutter 3.41.7 at `C:/Manual Installs/flutter/packages/flutter/lib/src`.
Frame twins are in `harness/flutter/test/frames/switchers_test.dart` and
`harness/incular/tests/frames/switchers.rs`.

## Coverage

| Flutter widget | Method and result | Open coverage |
| --- | --- | --- |
| `AnimatedSwitcher` | `switchers__switcher_overlapping_children`: 30 frames, all recorded fields match. | The transition-builder callback type differs; custom builders that use animation status or compose animation nodes are not covered. |
| `AnimatedCrossFade` | `switchers__cross_fade_reversed_different_sizes`: reversed while animating; first geometry difference at frame 3 / 48 ms. | The recorder does not compare paint order, clipping, or focus/semantics. |
| `AnimatedSize` | `switchers__animated_size_retarget`: child grows, then retargets mid-flight; first size difference at frame 11 / 176 ms. | Flutter exposes `reverseDuration`, but its 3.41.7 render path only calls `forward`; no reachable reverse timing difference was found. |
| `AnimatedList` | `switchers__animated_list_insert_size_transition`: inserts at index 1 with matching 300 ms `SizeTransition`; traces match. | Removal and overlapping insert/remove transitions are unexamined. |
| `SliverAnimatedList` | `switchers__sliver_animated_list_initial_layout`: initial two-row layout matches. | Insertion/removal transition and builder-driven effects are unexamined; Incular's item builder receives only an index, without Flutter's animation. |
| `AnimatedGrid` | `switchers__animated_grid_insert_size_transition`: inserts at index 1 with matching 300 ms `SizeTransition`; traces match. | Removal and rapid repeated mutations are unexamined. |
| `SliverAnimatedGrid` | `switchers__sliver_animated_grid_insert_size_transition`: inserts at index 1 with matching 300 ms `SizeTransition`; traces match. | Removal and rapid repeated mutations are unexamined. |
| `AnimatedModalBarrier` | `switchers__animated_modal_barrier_color_and_dismiss`: animated color and pointer dismissal callback match over 27 frames. | Semantic actions and a nullable color value are not represented by the frame recorder; Incular also has no nullable color animation type. |

## Findings

### switchers-001 — AnimatedCrossFade outgoing child keeps its narrow width

- **Priority / confidence:** P2 / medium-high.
- **Evidence:** In `switchers__cross_fade_reversed_different_sizes`, first divergence is frame 3 at 48 ms. The outgoing `cross_first` probe is 150×40 in Flutter and 90×40 in Incular. Its rect is `[325,280,475,320]` versus `[355,280,445,320]`; the incoming probe is also horizontally displaced in Incular.
- **Flutter:** `widgets/animated_cross_fade.dart:223-233` places the bottom child with `left: 0`, `right: 0`, and `top: 0`, so it takes the top child's width.
- **Incular:** `crates/incular-widgets/src/animated_cross_fade.rs:133-146` builds the corresponding `Positioned` child. The retained stack constraint path is `crates/incular-widgets/src/tree/layout/behavior/containers.rs:291-320`.
- **Impact:** During a cross-fade between unequal-width children, the outgoing child remains narrow and can appear horizontally offset instead of spanning the incoming child's width.
- **Cause:** The default layouts express the same opposing-edge constraints, but Incular does not deliver the effective width to the child under its transition wrappers in this scenario. The probe confirms the child keeps its explicit 90 px width where Flutter's positioned child is forced to 150 px.
- **Fix direction:** Trace the two-sided `Positioned` width constraint through the retained opacity/semantics wrappers; make the bottom subtree adopt the computed stack width as Flutter's `RenderStack` does.

### switchers-002 — AnimatedSize retarget uses the previous animation tick

- **Priority / confidence:** P2 / high.
- **Evidence:** In `switchers__animated_size_retarget`, phase 2 changes the child target from 180×100 to 120×64. At frame 10 / 160 ms both sides are 117.33×62.4. At frame 11 / 176 ms Flutter remains at 117.33×62.4 while Incular advances to 122.67×65.6, then animates back toward the new target. The max size difference is 5.333 px.
- **Flutter:** `rendering/animated_size.dart:246-271` lays out the current child, updates the state machine, then computes the animated size in the same `performLayout`; `_layoutStable` retargets from the current laid-out size at `:328-334`.
- **Incular:** `crates/incular-widgets/src/animated_size.rs:128-175` receives child-size changes through `child_laid_out`; `:179-233` builds the outer animated size around a `Widget::size_observer` child.
- **Impact:** A child retarget during an active size animation moves the outer box one frame farther along the old target before reversing, producing a visible geometry jump.
- **Cause:** Incular applies the new child-size observation after the frame's animation tick has already supplied the outer size. Flutter sees the changed child before it computes the box size for that frame and restarts from the prior displayed size.
- **Fix direction:** Make target-size changes participate in the same layout pass that computes the outer size, preserving the last displayed size as the new tween begin before applying the current tick.

### switchers-003 — AnimatedSwitcher builder receives a scalar instead of an animation

- **Priority / confidence:** P2 / high.
- **Flutter:** `widgets/animated_switcher.dart:53-54` defines the callback as `(Widget, Animation<double>)`.
- **Incular:** `crates/incular-widgets/src/animated_switcher.rs:16-17` defines it as `(Widget, f32)`.
- **Impact:** A straightforward fade can use the value, as the default transition does, but a ported custom builder cannot inspect animation status, pass the animation to another transition, or compose/listen to its animation graph.
- **Cause:** Incular reduces the transition input to the sampled fraction before invoking the builder.
- **Fix direction:** Pass a cloneable animation handle exposing value and status to the callback, matching the other Incular animation-transition APIs.

### switchers-004 — AnimatedSize reverseDuration is an API-only difference

- **Priority / confidence:** P3 / high.
- **Flutter:** `widgets/animated_size.dart:29-37, 74-78` accepts `reverseDuration` and forwards it to `RenderAnimatedSize`.
- **Incular:** `crates/incular-widgets/src/animated_size.rs:21-28, 30-69` stores only one duration and has no reverse-duration setter. Flutter's `rendering/animated_size.dart:309-374` only invokes `forward`/`forward(from: 0.0)` in the inspected size state machine; no `reverse()` call was found.
- **Impact:** This is a source-level configuration difference only in Flutter 3.41.7; the inspected widget path has no reverse run whose timing Incular could fail to match.
- **Cause:** Flutter exposes and stores `reverseDuration`, while the size state machine restarts and resumes forward runs only.
- **Fix direction:** No behavior change is indicated by this audit. Add the property only if source-level API compatibility is a requirement.

### switchers-005 — AnimatedModalBarrier cannot animate to a null color

- **Priority / confidence:** P2 / high.
- **Flutter:** `widgets/modal_barrier.dart:289-300` accepts `Animation<Color?>`.
- **Incular:** `crates/incular-widgets/src/navigation/barrier.rs:17-24, 54-67` requires `Animation<Color>`.
- **Impact:** Flutter callers can animate the barrier between a fill and no fill while keeping the modal barrier's interaction behavior. Incular's typed API cannot express the no-fill endpoint as `None`.
- **Cause:** Incular's barrier color animation is non-nullable.
- **Fix direction:** Support an optional color animation value and preserve the barrier's hit-testing and semantics when the sampled color is `None`.

### switchers-006 — SliverAnimatedList builder omits the item animation

- **Priority / confidence:** P2 / high.
- **Flutter:** `widgets/animated_scroll_view.dart:899-900` defines `AnimatedItemBuilder` with `Animation<double>`.
- **Incular:** `crates/incular-widgets/src/scrolling/sliver_lists.rs:958-974` stores a `Fn(usize) -> Widget`; it passes no progress or animation handle to the item builder.
- **Impact:** Incular's sliver list can animate its own row extent, but user item builders cannot add matching per-item fade, slide, or other effects during insertion/removal.
- **Cause:** The Incular sliver implementation owns the extent animation internally and exposes only the index to the child builder.
- **Fix direction:** Expose the retained item's animation value or handle to the builder, consistent with Flutter and Incular's other animated collection builders.

## Pixel and harness notes

No pixel-only proposal is needed for the two observed visual divergences: the frame recorder already captures their geometry. A semantics-focused test is needed to compare nullable barrier color while preserving dismissal semantics. A future sliver-list insertion twin should be added once Incular exposes item animation progress; the current baseline twin verifies initial layout only.

## Validation

- `cargo fmt -p incular-parity-harness`: passed.
- `bash harness/run-frames.sh switchers__`: the requested `bash.exe` resolved to the Windows WSL stub, so the same script was run with Git for Windows' Bash at `C:/Program Files/Git/bin/bash.exe`; all eight Flutter tests passed. Frame results: 6 matches, 2 divergences (`switchers-001`, `switchers-002`).
- `flutter analyze`: passed (`No issues found`).
- `cargo clippy -p incular-parity-harness --all-targets -- -D warnings`: blocked by an existing unrelated `clippy::useless_conversion` in `harness/incular/tests/frames/anim_core.rs:282`; no changes were made there.

## Status at clean stop

- Done: coverage rows rewritten (`coverage/switchers.jsonl`); new scenarios `switchers__animated_list_remove_shifts_items` (matches except switchers-007), `switchers__switcher_same_key_no_transition`, `switchers__switcher_reverse_duration`, `switchers__switcher_layout_builder_top_left`, `switchers__animated_size_top_left_alignment` (all match); switchers-001 revised (now a one-frame outer-size lag, same mechanism as switchers-002; the original 90 px width claim no longer reproduces); switchers-002 to 006 revalidated; new finding switchers-007 (P3, removed item retained one frame at zero size).
- Not done: findings JSONL not normalized and switchers-007 not written to JSONL; SliverAnimatedList insert/remove needs a fake-clock harness capability; removeAllItems and AnimatedGrid removal not frame-tested.

## Resume update

- JSONL normalized to the brief schema; switchers-001 revised (one-frame outer-size lag, shared with switchers-002); switchers-007 added (P3).
- SliverAnimatedList insert/remove twin: not written. Missing capability: (a) the mount closure cannot read the fake clock. FrameHarness keeps `start` private and exposes only `elapsed()` (harness/incular/src/lib.rs:176), so a closure cannot call `insert_at(index, now)` with the frame time; `h.pump` advances time but does not hand it to the builder. (b) `SliverAnimatedListController` keeps each entry's animation private (sliver_lists.rs:678-700), unlike `AnimatedCollectionController::animation_for_id`, so the twin cannot restart the entry on the next tick the way the AnimatedList twin does. Request: a public `FrameHarness::now()` (or time passed to the mount closure) plus a public `animation_for_id` on `SliverAnimatedListController`.
- Frame runs at a19bf487 after the recorder change: implicit 16/19 (diverge: implicit-001, implicit-002, implicit-007), switchers 10/13 (diverge: switchers-001, switchers-002, switchers-007). transitions 8/9 (diverge: implicit-002 cross-reference). The transitions run hit a compile error in another worker's `text_test.dart` (line 751) on the final attempt, so its Flutter traces come from the earlier successful run.
