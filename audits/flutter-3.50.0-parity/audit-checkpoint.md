# Audit continuation checkpoint

Status: **open**. This is audit and evidence only, not authorization to change
framework behavior. Pinned Flutter 3.50.0 and Incular source are listed in
[README](README.md); report the implementation revision actually tested if it
changes. Read [coverage ledger](coverage-ledger.md), [contract review](contract-review.md),
and [focused comparisons](focused-comparisons.md) before duplicating a probe.

## Verification backlog

Each track is open. Select a bounded upstream test file or named group plus its
actual Incular behavior path. Record reviewed case IDs/paths and resulting
dispositions, and keep all other rows open. A source-only gap and a paired run
have different evidence. No assumption from matching names or baseline 3.41.7.

| Track | Next high-signal cases | Additional closure conditions |
| --- | --- | --- |
| animation | Repeat curve, callbacks removing siblings, nullable and large-int tweens | Curves, status/action order, endpoint, looping, async and accessibility variants |
| foundation | Contextual error and listener mutation | Utilities, diagnostics, errors, comparison/collection contracts and app-facing notifier equivalents |
| gestures | Rotated axis arbitration and synthesized velocity | Each recognizer, nested arenas, mouse/touch/trackpad/stylus, thresholds and dispatcher/capture lifetimes |
| painting | Width-zero hairline and draw-path attribute preservation | Neutral brush/filters/text shaping, image streams/codecs/cache, DPR transforms and color spaces |
| physics | Fractional/large bounded friction near tolerance | All simulations at endpoint and invalid inputs, signed velocity and precision policy |
| rendering | Custom clip hit and clip-mode pixels | Constraints/intrinsic/dry/baseline, dirty/repaint/compositing, layer lookup, custom tree capability |
| scheduler | Muted ticker and callback removal during an active phase | Timing, suspension, task priorities, post-frame/first-frame/reentry and multi-window |
| semantics | Nested BlockSemantics and focus/action routing | Semantics tests by contribution, matrix for desktop/web/Android/iOS bridges and AT |
| services | Restoration removal and stale input connection state | Fresh clipboard/IME, key maps/host services, assets/bundles, restoration, codecs and capability support |
| widget_previews | CLI detection, wrapper order, and updates | Annotation, exported API, preview discovery/scaffold tested individually or recorded unsupported |
| widgets | RTL scrolling, safe-area nesting, hidden retained focus | Layout lanes, async/state/restoration, navigation/overlay, input/media/platform composition, and remaining neutral widgets |

Use pinned upstream tests and small behavioral probes, not full workspace
runs. Allow known intentional Incular differences only with a written
disposition and exact app-facing consequence. Record whether each device or GPU
path ran. Set estimates against a named denominator; report registration coverage
separately from paired comparisons. If local access is down, record the last
verified state without counting the missing run as passing.

## Latest paired slice

[C07–C08: retained Visibility and nested SafeArea](visibility-safe-area.md):
two new paired mismatches, one paired positive control. Combined with
C01–C06, eight custom paired mismatches are documented. This is not upstream
case closure. Next: rotate to gestures (rotated axis arbitration and
synthesized velocity), then painting or another under-reviewed track.

### Async stream delivery edge cases (open)

Compare pinned Dart synchronous broadcast and Incular UiStreamController:
listener A drops B during event 1 (B should not receive it); A tries a
reentrant add(2) during event 1 (check rejection or preserved subscriber
order); A tries add after Done is delivered (no post-terminal data).
Include ordinary multi-listener delivery and close controls. Rust source:
crates/incular-widgets/src/async_builders.rs:350–351,379–404,458–469.
Dart sources in the pinned SDK lib/async: stream.dart:107–113,
stream_controller.dart:123–166, broadcast_stream_controller.dart:266–275,306–342.
These source leads are not runtime results. Keep the next gestures slice
as planned, and include these in the later async rotation.

### C09–C10: gesture arbitration and synthesized scale velocity

[Paired gesture tests](gesture-comparisons.md) reproduced two more mismatches,
bringing the custom paired mismatch count to ten. Both had matching ordinary
or below-threshold controls. Upstream formal verification denominator remains
separate. Next slice: painting/rendering, zero-width hairline in pixels and
drawPath Paint attributes; async lifecycle remains queued.

### C11–C17: raster and synchronous broadcast

[Windows raster readback](raster-comparisons.md) paired four mismatches at
DPR 1 and 2, plus an ordinary path positive control.
[Synchronous broadcast](stream-comparisons.md) paired three mismatches
with a positive ordinary delivery/close control. Running total: 17 custom
paired mismatch cases, not upstream registration sign-off.
Next rotate into services (restoration removal and retained writers);
continue physics, foundation and widget-preview below that in the queue.

### C18–C20: persistence removal and simulation precision

[Paired persistence and physics tests](restoration-physics-comparisons.md)
confirmed three mismatches; running total 20 custom paired mismatches.
Next: foundation listener lifecycle as a positive control and animation
tween endpoints/precision. Keep the preview toolchain track open.

### C21–C22: tween endpoints, listener control, preview reference

[Listener, tween and preview evidence](listener-tween-preview-comparisons.md):
two more paired mismatches (running total 22), one paired listener control
passed, and two reference preview cases passed with no matching Incular
preview API. Do not count reference-only cases as paired failures.
Next: semantics blocker traversal, then refine the upstream ledger to
account for registrations through verified helper wrappers.

### C23 and upstream BlockSemantics dispositions

[Semantics blockers](block-semantics-comparisons.md): Padding exposes the
earlier sibling in Incular. Running custom paired mismatches: 23.
The first two exact upstream cases have dispositions (match, mismatch);
a third ran on Flutter only. See [upstream dispositions](upstream-dispositions.tsv),
which overlays the generated registration catalog without mutating it.
Coverage denominator expanded to 8,965 with 216 verified testGesture
wrapper call sites. Continue with navigation reconciliation.

### C24–C25: declarative route reconciliation

[Navigation comparisons](navigation-comparisons.md) confirmed unkeyed route
remount/focus loss and a dropped pageless entry over a retained keyed page.
Running total 25 custom paired mismatches. The keyed route control passed.
Next: custom clip hit testing and existing focused frame harness cases.

### C26–C27: path hit tests and horizontal direction

[Clip and scroll comparisons](clip-scroll-comparisons.md) paired two more
mismatches; total 27 custom paired mismatch cases. Interior clip and LTR
controls passed. Existing harness rechecks are a separate denominator.

### Existing frame harness: first strict 3.50 rechecks

[Strict rechecks](frame-rechecks.md) dispositioned 11/253 existing frame
fixtures (4.35%): ten match, and the fade-pop timeline diverges.
[Harness dispositions](harness-dispositions.tsv) holds exact fixture keys.
This denominator is independent of the 8,965 upstream registration sites.
Next: continue narrowly grouped frame comparisons and word-spacing/formatter
probes; preserve verification details for divergences.

### C28–C30: text shaping and formatter behavior

[Text comparisons](text-format-comparisons.md) reproduced three additional
mismatches: word spacing in warm/cold text layout, digit filtering caret
remap, and length enforcement dropping an old suffix. Total: 30 original
custom paired mismatches. Exact established fixture dispositions remain
separate.

### Existing frame harness: second and third strict batches

[Batch results and runner](frame-rechecks-batch2.md) add nine matching
fixtures. Total 20/253 existing frame fixtures (7.91%): 19 match, one
mismatch. A repeat of fade-pop reproduced the existing divergence and
does not count twice. Use the runner only for small reviewed batches.

### C31–C33: loose layout and baseline

[Loose layout probes](loose-layout-comparisons.md) confirmed OverflowBox,
FittedBox and Baseline divergences under looser inputs than the matched
existing harness controls. Total 33 custom paired mismatches. Next rotate
into animation, scheduler, physics and additional small frame batches.

### C34–C35: animation controller notification lifecycle

[Controller lifecycle](controller-lifecycle-comparisons.md) paired two
mismatches. Ordinary repeat after a per-run curve override matched, and
is distinct from Incular's persistent curve extension. Total 35 custom
paired mismatch cases.

### Existing frame harness: batches four through six

[Strict frame rechecks](frame-rechecks-batches4-6.md) add 15 matched
fixtures. Current verified count 35/253 (13.83%), 34 match, one mismatch.

### Exact upstream physics dispositions

[Physics translations](upstream-physics-comparisons.md) add 12 paired
upstream registrations: seven match, five differ (mostly strict numeric
precision plus zero-duration policy). Overall 14/8,965 upstream paired,
one additional reference-only; 35/253 existing harness and 35 original
custom mismatches are separate counts. Continue small scroll/sliver frame
batches and the next unresolved upstream file; do not count unrun cases.

### Existing frame harness: batches seven and eight

[Strict frame rechecks](frame-rechecks-batches7-8.md) add 10 paired frames
fixtures (7 match, 3 differ). Total 45/253 (17.79%), 41 match and four
differ. Next: nested scrolling, viewports/overlays, follow up the undo
grouping mismatch, and continue upstream scenario dispositions.

### Existing frame harness: batch nine

[Strict frame rechecks](frame-rechecks-batch9.md) add five paired fixtures
(4 match, 1 mismatch). Total 50/253 (19.76%), 45 match and five differ.
Viewport center/anchor differs on the minimum reported scroll extent.

### C36–C39: hue seams and simulation clamp edges

[Color and clamp probes](color-clamp-comparisons.md) confirmed two hue
interpolation differences, NaN clamp policy, and missing debug-only
bounds validation. Total 39 custom paired mismatch cases. Finite clamp
and wrapped completion provide controls.

### Existing frame harness: batch ten and upstream finite clamp controls

[Five frame rechecks](frame-rechecks-batch10.md) all match. Total 55/253
(21.74%), 50 match and five differ. Two more
[upstream physics pairs](upstream-physics-comparisons.md#upstream-clamped-simulation-controls)
passed on both implementations: now 16/8,965 paired (10 match, six differ),
plus one reference-only.

### C40: undo history grouping, plus a control for the frame mismatch

[Timing controls](undo-history-comparisons.md) reproduce the early
deletion divergence, show that both sides match after seeding and making
one edit, and separately demonstrate that two rapid edits undo as a group
on Flutter but individually on Incular. Total 40 custom paired mismatch
cases; the early frame mismatch is not counted again in this total.

### Existing frame harness: batch eleven

[Five frame rechecks](frame-rechecks-batch11.md) bring the total to
60/253 (23.72%), 54 match and six differ. The additional difference is
unset versus true semantics enabled state; the tap action still matches.

### Existing frame harness: batch twelve

[Navigation and semantics rechecks](frame-rechecks-batch12.md) bring the
total to 63/253 valid paired outcomes (24.90%), 55 match and eight differ at that point. Notable results:
the fade-push timeline, the non-equivalent Hero fixture
and opacity in the PopScope fixture (back-block values matched). The repeated overlay match is not counted again.

### Existing frame harness: batch thirteen and Hero fixture correction

[Focus and input rechecks](frame-rechecks-batch13.md) add five matches.
Only Incular configured a Hero flight controller in the original push
fixture; the initial raw divergence is retained as non-equivalent, not
counted as product behavior. The coverage script excludes such fixtures
from valid paired outcomes.

### Executable regression backfill

The [test backfill ledger](test-backfill.tsv) maps C01–C40 and B01–B06 to
committed Rust regression tests. All 40 prior custom paired mismatches
and all six prior Incular source-backed behavior probes are backfilled.
Each new test target was compiled and run narrowly; the regression
assertions intentionally fail on the audited implementation. Positive
controls remain in the corresponding targets. For C11–C14, the existing
Windows live-test gate is required for genuine wgpu readback at DPR 1/2.

This is 46/46 of the already confirmed backfill inventory, not 100% of the
parity audit. Existing frame registrations and upstream dispositions are
measured independently. Keep adding durable tests for reproducible cases,
verify fixture inputs before treating strict frame differences as product
bugs, and leave implementation for later. No full workspace tests or
runtime/wgpu implementation changes were made.

### More upstream test targets and frame comparability checks

[Asset and Newton pairs](upstream-assets-newton-comparisons.md) are now
committed test targets. They add 12 upstream pairs, 10 match and two
differ; current upstream coverage 31/8,965. [Focus/input batch
fourteen](frame-rechecks-batch14.md) finds one stale external focus node
after keyboard scrolling, and four matches. [Navigation re-review](navigation-fixture-comparability.md) excludes five non-equivalent
fixtures from valid pairs. Results to this point: 74 raw frame runs,
69 valid paired (64 matched, five differed); five need fixture repair.

### Batch fifteen and retained scroll regression targets

[Five animation and widgets frame checks](frame-rechecks-batch15.md)
matched. Valid paired frame coverage is now 74/253 (29.25%), 69 match
and five differ. Five additional runs need navigation fixture repair.
The centered SliverList extent estimate and stale external FocusNode
after PageDown now have dedicated, runnable Rust regressions:
`flutter_sliver_anchor_parity` and `flutter_scroll_focus_parity`
in incular-runtime. They fail at the confirmed estimate and focus checks.
