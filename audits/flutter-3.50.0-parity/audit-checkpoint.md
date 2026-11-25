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

### Exact harness selection

[Two overlapping gesture names](frame-exact-selection.md) now run
independently and both match. Current valid frame coverage is 76/253
(30.04%), 71 match and five differ, plus five non-equivalent fixtures.
Use the exact runner for subsequent small frame checks.

### Further executable frame and runtime checks

[Batches sixteen through eighteen and repaired Hero](frame-rechecks-batches16-18.md)
reach 92/253 valid paired frame registrations (36.36%): 86 match and six
differ. Five further registrations have non-equivalent probes. Actual
Hero push shuttles matched; pop had no Incular return shuttle. Added
focused executable regressions for unset enabled semantics and restoration
ownership handoff, with matching explicit removal and lease controls.

### Paint, transitions, navigation event controls, and overscroll

[New strict comparisons](frame-paint-scroll-rechecks.md) reach 100/253
valid paired frame registrations (39.53%): 93 match and seven differ.
Three other registrations remain non-equivalent. The two loop-expanded
overscroll fixtures together count once; missing UserScroll events occur
in both. Repaired observer order and blocked-back fixtures match without
relying on a covered-route opacity probe.

### Repaired fixtures, text checks, and additional upstream test targets

[New frame results](frame-repaired-and-text.md) and the repaired IndexedStack
and fade tests bring the frozen harness registration ledger to 108/253
valid pairs (42.69%): 101 match, seven differ. Fresh repair comparisons
use actual paint assertions or equally authored transitions. Historical
catalog line numbers identify the original registration sites; newer
fixture edits preserve their IDs. The real RTL IndexedStack and missing
touch UserScroll events have separate committed Rust regressions.

[Foundation and unbounded-scroll translations](upstream-foundation-scroll-comparisons.md)
bring the upstream ledger to 37/8,965 paired (29 match, eight differ),
with three additional reference-only outcomes.

### Curves and tweens

[Five pinned strict samples](frame-curves-and-tweens.md) all match.
Current checkpoint: 113/253 valid paired frame registrations (44.66%),
106 match and seven differ.

### Focused switcher and animated-child fixtures

[Five strict switcher checks](frame-switchers-recheck.md) match. Valid
paired frame registrations: 118/253 (46.64%), 111 match and seven differ.
Upstream paired registrations: 37/8,965.

### Input, retained layout, and platform lifecycle

[Three strict groups](frame-input-layout-rechecks.md) add 15 matches:
133/253 comparable frame registrations (52.57%), 126 match and seven differ.
[Five official checks](upstream-lifecycle-clipboard-comparisons.md) have
committed Rust targets; typed clipboard controls match, ticker suspension
and start-before-resume fail at the intended assertions. Upstream count
42/8,965, 32 match and ten differ (plus three reference-only).
Earlier Flutter reference expectations are now being preserved as real
named test files under `reference-probes/test`, with a pinned runner.

### Layout, focus/input, and semantics controls

[Ten more frame pairs](frame-layout-focus-semantics.md) match. Valid frame
registrations now 143/253 (56.52%), 136 match and seven differ. The
original focus/input frame file is fully dispositioned (17 of 18 match).
All previously reproduced custom Flutter expectations with existing
standalone comparison files now have runnable named reference probes
in the audit package; shipping implementation remains untouched.

### Sliver and scroll frame rechecks, shortcut dispatch

[New frame groups](frame-sliver-scroll-rechecks.md) add eight comparable
registrations (six match, two differ): 151/253 (59.68%), 142 match and
nine differ. KeepAlive and PageView's separate raw geometry/retention
differences remain under fixture review and are not counted.
[Shortcut results](upstream-shortcut-comparisons.md) bring upstream
pairs to 44/8,965 (34 match, ten differ), with six reference-only.
Committed standalone Flutter fixtures now preserve expected behavior
in source tests as well as the existing harness; no runtime fixes.

### Sliver viewport and header fixtures

[Five more strict sliver comparisons](frame-sliver-viewport-rechecks.md)
match. Valid paired frames: 156/253 (61.66%), 147 match and nine differ.
Upstream: 44/8,965. Now 23 standalone Flutter reference fixture files
are committed; continue keep-alive paint, PageView retention, and the
remaining retained layout/text/physics fixture groups.

### Retained scrolling, text, and implicit rechecks

[New strict comparisons](frame-retained-text-implicit-rechecks.md)
bring valid frame coverage to 171/253 (67.59%), 161 match and ten differ.
Dedicated paired tests now verify real parked KeepAlive paint and
PageView boundary teardown differences. Retained row identity on the
ordinary clipped KeepAlive path matches after fixture repair.

### Physics/layout fixtures and editor/scheduler event order

[More frame rechecks](frame-physics-layout-editing.md) bring valid frames to 183/253 (72.33%):
171 match and 12 differ. The original raw Enter difference now has
pinned direct tests; an explicit IME Done matches. The alternate undo
fixture repeats the known grouping problem. [Scheduler phases](scheduler-callback-phases-comparisons.md)
produce two dedicated red tests and one upstream positive pairing:
45/8,965 upstream pairs. No implementation fixes were made.

### More text controls and verified offscreen accessibility regression

[Nine text frame controls](frame-editable-sizing-and-input.md) match. Current coverage:
192/253 valid frame registrations (75.89%), 180 match and 12 differ.
The upstream [sliver semantics test](offscreen-sliver-semantics-comparisons.md) exposes two separately
reproduced differences, at first layout and at normalized offset.
Upstream paired registrations: 46/8,965 (35 match, 11 differ).

### Spring, bounded physics, and fling frame controls

[Nine more strict comparisons](frame-constraint-and-fling-rechecks.md) matched.
Valid frame registrations: 201/253 (79.45%), 189 match and 12 differ.
Upstream paired registrations: 46/8,965. Two raw captures currently
under equivalence review are Unicode selection units and zero-speed
bouncing spring return.

### Comparable text units and remaining spring dispatch

[Seven more frame dispositions](frame-text-units-and-bounce.md) bring valid frame coverage to
208/253 (82.21%), 195 match and 13 differ. Core animation, composition,
text and physics frame registration files are fully dispositioned.
Unicode caret units were normalized in the fixtures only; zero-speed
bouncing beyond a bound shows a real position, velocity and completion gap.

### Implicit, layout, semantics, and upstream rubber band controls

[Twenty more frame pairs](frame-implicit-layout-semantics.md) match. Frame coverage:
228/253 (90.12%), 215 match and 13 differ. Two [upstream rubber-band
cases](upstream-rubber-band-comparisons.md) have numeric mismatches but no
equivalent public concrete simulation query in Rust; they are partial
and excluded from the 46 fully paired upstream registrations.

### Transition and widget samples; border, constraint and input tests

[Twenty-four more frame checks](frame-switchers-transitions-samples.md) matched.
Frame coverage is 252/253 valid pairs (99.60%), 239 match and 13 differ.
Only the ListWheel gesture/default-physics fixture remains under
review. [Eight upstream pairs](upstream-border-constraints-formatters.md) plus
six partial ports have committed Rust test targets. Full upstream
pair count is 54/8,965 (40 match, 14 differ), separately from
partials or reference-only results. Production code is unchanged.

### Animation controller upstream ports and ListWheel proof

[Four original controller checks](upstream-animation-controller-comparisons.md) bring the
fully paired upstream ledger to 58/8,965 (43 match, 15 differ).
A dedicated controlled ListWheel Rust regression confirms a real
touch routing gap with both sides explicitly clamping. Programmatic
jump works on both. The standalone and full frame fixtures are being
paired with the same inputs; the frozen frame ledger remains 252/253
valid until the controlled gesture trace is verified.

### Frame fixture milestone; upstream audit continues

[Final controlled ListWheel fixture](frame-list-wheel-and-coverage-milestone.md) reproduces missing
touch scrolling after matching physics and attachment. All 253 static
frame registrations now have valid outcomes: 239 match, 14 differ.
[Five original Wrap cases](upstream-wrap-geometry-comparisons.md) add five
positive pairs. Upstream: 63/8,965 fully paired, 48 match and 15 differ,
plus eight partial and six reference-only. Continue the upstream audit
in bounded behavior families; do not treat 100% fixture coverage as a
complete equivalence verdict or start implementation fixes.

### Upstream pools: geometry, text state and gestures

[Six more full behavioral pairs](upstream-geometry-input-gestures.md) bring the upstream ledger
to 69/8,965 (52 match, 17 differ); the partial (15) and reference-only
(9) groups are separate. Committed paired regressions cover relative
alignment origins, follower scale inheritance, selection preservation,
and direct arena winner finality. Continue original test families from
the upstream ledger; the 253 existing frame registrations are complete.

### Completed upstream text replacement group

[Seven additional exact behavior pairs](upstream-text-replacement-boundaries.md) cover the remaining
original replacement group. Two match, five reproduce the same
selection-collapse bug under distinct range boundaries. Upstream now
76/8,965 fully paired, 54 match and 22 differ; frame milestone unchanged.

### Upstream pointer/wheel and measured gradient sampling

[Seven complete upstream pairs](upstream-pointer-wheel-gradients.md) add three
matches and four differences (two are paint lifecycle only). Upstream:
83/8,965 paired (57 match, 26 differ), plus 15 partial and 12
reference-only. The committed real Windows wgpu gradient probe
reproduces a transparent-stop color loss across three geometries;
opaque controls match. No production changes.

### Focus, radius, curve and preview checks

[Recent upstream groups](upstream-focus-corners-curves-previews.md) bring complete
pairs to 92/8,965 (64 match and 28 differ), with 22 partial and 15
reference-only. New tests cover replacement autofocus, actual
elliptical raster shape, image cropping, unsupported spline inputs,
and inherited text style resets. Preview manifest behavior is
reference-only because Incular lacks the matching tool surface.
Audit only; implementation and full workspace runs remain untouched.

### Weighted sequence, focus policy and preview boundary

[Three original TweenSequence pairs](upstream-focus-policy-tween-dtd.md) match. Upstream:
95/8,965 complete pairs (67 match, 28 differ), 26 partial and 22
reference-only. New tests expose inaccurate ancestor/blocked-node focus
flags and a finite/infinite constraint interpolation difference.
Four hermetic preview DTD results are reference-only, not Rust pairs.

### Clip hit testing and hidden widget focus

[Eight upstream dispositions](upstream-clip-visibility-focus.md) bring
complete pairs to 99/8,965 (67 match, 32 differ), with 30 partial and
22 reference-only. Clipped-away taps get through four neutral clip
variants. Hidden focus tests distinguish stale detached-node state
from actual hidden keyboard delivery.

### Mouse buttons, absorber semantics, edit deltas and RawImage

[Fifteen upstream dispositions](upstream-tap-absorber-delta-and-raw-image.md)
bring complete pairs to 108/8,965 (72 match, 36 differ), with 36 partial
and 22 reference-only. Actual Windows wgpu pixels confirm a separate
nine-slice RawImage default difference, with all explicit-fit controls
matching. No production code changed.

### Route-current input, aspect ratio, and long-press button mask

[Nine upstream dispositions](upstream-current-route-aspect-and-long-press.md)
bring complete pairs to 113/8,965 (75 match, 38 differ), with 40 partial
and 22 reference-only. The new tests distinguish callbacks on a covered
route, aspect ratio under infinite width, and button changes on either
side of long-press acceptance.

### Opacity, hover, flex, notifier and ticker futures

[Seventeen upstream dispositions](upstream-opacity-hover-flex-notifier-futures.md)
bring complete pairs to 125/8,965 (85 match, 40 differ), with 45
partial and 22 reference-only. Actual opacity raster masks match,
but Incular invokes the custom painter at effectively zero alpha.
New lifecycle regressions distinguish incorrect local mouse
coordinates, interrupted callback dispatch, and an uncancelled
controller future.

### Semantic headings, singular fitted pose, clamping and time dilation

[Fourteen upstream dispositions](upstream-semantic-headings-clamping-and-dilation.md)
bring complete pairs to 131/8,965 (91 match, 40 differ), with 53
partial and 22 reference-only. New tests expose lost merged heading
levels and semantics from a singular fit; clamping differences are
isolated to public precision at strict tolerances. The scheduler
tests match both original fast and slow time-dilation cases.

### Fit updates, overlays, scrolling, notifications and clipboard

[Twenty-five upstream dispositions](upstream-fit-overlays-scroll-notifications-clipboard.md)
bring complete pairs to 147/8,965 (106 match, 41 differ), with 62
partial and 22 reference-only. New differences include retained
FittedBox size on a fit change and Ctrl+A on obscured read-only
text. Actual Windows wgpu readback finds a separate ColorFiltered
bounds defect with a nonzero alpha translation.

### Dismissible timing, direction and pending confirmation

[Seven upstream dispositions](upstream-dismissible-direction-and-confirmation.md)
bring complete pairs to 148/8,965 (107 match, 41 differ), with 65
partial and 25 reference-only. Pointer direction, thresholds, and
no-resize callbacks are tested. Awaiting a confirmation decision
has no equivalent public Incular hook.

### Curves, intrinsic layout and overlay-style sampling

[Twelve upstream dispositions](upstream-curves-intrinsic-layout-and-system-overlay.md)
bring complete pairs to 153/8,965 (112 match, 41 differ), with
72 partial and 25 reference-only. Five complete curve tests match,
and negative intrinsic step inputs still differ. Status/navigation
annotation sampling matches at the host payload layer.

### Repeat validation, explicit focus, restoration, flex and matrix sampling

[Twenty upstream dispositions](upstream-repeat-focus-restoration-flex-and-image-filter.md)
bring complete pairs to 160/8,965 (118 match, 42 differ), with 83
partial and 27 reference-only. New independent probes reproduce
zero-count infinite repeat, direct focus lost after traversal exclusion,
different restoration write timing, and default matrix sampling.

### RawTooltip input modes and held-press timing

[Five upstream pairs](upstream-raw-tooltip-trigger-and-hold.md) bring
complete pairs to 165/8,965 (122 match, 43 differ). RawTooltip tap,
manual, and release controls match; the held long press mounts the
bubble too late and misses the compared dismissal deadline.

### Tweens, transform hits, ownership and lifecycle events

[Fourteen upstream dispositions](upstream-tween-transform-and-lifecycle.md)
bring complete pairs to 174/8,965 (131 match, 43 differ), with 88
partial and 27 reference-only. A separate pinned native probe
finds that ClipRRect antialiased modes produce binary coverage on
Windows wgpu.

### Escape, modal input, key listeners and Bézier bounds

[Ten upstream dispositions](upstream-tooltip-escape-modal-keys-and-path-bounds.md)
bring complete pairs to 179/8,965 (134 match, 45 differ), with 93
partial and 27 reference-only. New tests expose Escape leaving
tooltips open, secondary release not dismissing the modal, and
tight rather than control-point Bézier bounds on the public API.

### Custom layout and asset cache invalidation

[Six upstream dispositions](upstream-custom-layout-and-asset-invalidation.md)
bring complete pairs to 182/8,965 (136 match, 46 differ), with 96
partial and 27 reference-only. Custom layout calls the wrong
delegate to decide whether a retained replacement relayouts.

### Raw menu closure, animation notification channels and shadow colors

[Twelve upstream dispositions](upstream-raw-menu-animation-observer-and-shadow.md)
bring complete pairs to 186/8,965 (139 match, 47 differ), with 104
partial and 27 reference-only. Committed tests pin old menu control
ownership, child-before-parent closure, train/proxy listener
effects, and the color clamp at extrapolation.

### LimitedBox geometry and BoxDecoration blend metadata

[Six upstream dispositions](upstream-limited-box-and-decoration-contracts.md)
bring complete pairs to 188/8,965 (141 match, 47 differ), with 108
partial and 27 reference-only. Two separately verified decoration
regressions concern self-interpolation and image-only validation.

### AnimatedSize, overflow constraints, controller runs and navigation events

[Sixteen upstream dispositions](upstream-animated-size-overflow-controller-and-navigation.md)
bring complete pairs to 196/8,965 (148 match, 48 differ), with
116 partial and 27 reference-only. New regressions pin unwanted
shrink clipping, child constraints, and removal notifications.

### Scrollable single-child reveal on both axes

[Five exact upstream pairs](upstream-ensure-visible-single-child.md)
all match, including reverse visibility policies, an animated reveal
and a rotated child. Complete pairs reach 201/8,965 (153 match,
48 differ); partial and reference-only counts are unchanged.

### Nested and lazy-list scrollable reveal

[Three more exact pairs](upstream-ensure-visible-nested-and-lazy.md)
match. Complete pairs reach 204/8,965 (156 match, 48 differ), with
116 partial and 27 reference-only. The nested 7×7 grid and all
ordinary ListView alignment/animation samples agree.

### AnimatedSwitcher rapid replacement and eviction

[Five upstream dispositions](upstream-animated-switcher-retained-lifetime.md)
bring complete pairs to 207/8,965 (158 match, 49 differ), with
118 partial and 27 reference-only. Opacity samples match but
the replaced intermediate child can remain mounted too long.

### AnimatedCrossFade focus and interaction

[Five upstream dispositions](upstream-cross-fade-focus-and-input.md)
bring complete pairs to 210/8,965 (160 match, 50 differ), with 119
partial and 28 reference-only. Default bottom focus is blocked by
the host correctly, but the inactive node incorrectly claims focus.

### Switcher builder and same-key reentry

[Three upstream dispositions](upstream-animated-switcher-builder-and-key-reentry.md)
bring complete pairs to 212/8,965 (161 match, 51 differ), with
120 partial and 28 reference-only. A reentered key can leave
two mounted children after the original animation window.

### Implicit animation completion timings

[Five exact callback sequences](upstream-implicit-animation-completion.md)
match, including the 1 ms boundary. Complete pairs reach
217/8,965 (166 match, 51 differ).

### Implicit transition samples and rebuild boundary

[Five exact effect comparisons](upstream-implicit-transition-samples.md)
match, including caller build counts. Complete pairs reach 222/8,965
(171 match, 51 differ); 120 partial and 28 reference-only remain.

### Additional implicit effect completion checks

[Five more callback sequences](upstream-implicit-effects-and-fractional-completion.md)
match. Complete pairs reach 227/8,965 (176 match, 51 differ).

### Positioned, physical model and tween completion

[All 14 focused implicit callback sequences](upstream-implicit-effects-and-fractional-completion.md)
now match. Complete pairs reach 231/8,965 (180 match, 51 differ).

### Flow delegate repaint, opacity and input order

[Six upstream dispositions](upstream-flow-paint-and-routing.md)
bring complete pairs to 232/8,965 (181 match, 51 differ), with
123 partial and 30 reference-only. Flow ignores a retained
delegate's external repaint change and ignores requested
ordering when choosing a hit in overlapped children.

### Multi-child relayout and child-count validation

[Nine upstream dispositions](upstream-multi-child-layout-contract.md)
bring complete pairs to 239/8,965 (184 match, 55 differ), with
125 partial and 30 reference-only. Duplicate and omitted child
layouts are silently accepted instead of rejected.

### Multi-child invalid identity and constraint diagnostics

[Four upstream dispositions](upstream-multi-child-layout-contract.md)
bring complete pairs to 240/8,965 (184 match, 56 differ), with
128 partial and 30 reference-only. All 13 registrations in this
original file have dispositions.

### Wrap constraints, overflow boundary and intrinsic height

[Five upstream dispositions](upstream-wrap-constraints-and-intrinsic-height.md)
bring complete pairs to 244/8,965 (188 match, 56 differ), with
129 partial and 30 reference-only. Intrinsic height, exact-width
and one-pixel overflow match; configurable Wrap clipping is unpaired.

### Wrap run alignment and vertical intrinsic sizing

[Five upstream dispositions](upstream-wrap-run-axes-and-vertical-intrinsics.md)
bring complete pairs to 248/8,965 (192 match, 56 differ), with
130 partial and 30 reference-only. A standalone geometry probe
isolates vertical intrinsic width at 80 instead of 188.

### Basic Wrap matrices, baseline and remaining references

[Seven upstream dispositions](upstream-wrap-baseline-and-final-references.md)
bring complete pairs to 251/8,965 (195 match, 56 differ), with
131 partial and 33 reference-only. Wrap loses the child's
alphabetic baseline even when direct Baseline layout works.

### Table direction, row shape and intrinsic width

[Ten upstream dispositions](upstream-table-direction-rows-and-intrinsics.md)
bring complete pairs to 258/8,965 (201 match, 57 differ), with
132 partial and 35 reference-only. Table ignores ambient
RTL column ordering and accepts uneven rows.

### Table state and semantics; LayoutBuilder baseline

[Six upstream dispositions](upstream-table-retention-and-layoutbuilder-baseline.md)
bring complete pairs to 261/8,965 (203 match, 58 differ), with
134 partial and 36 reference-only. Row changes reuse an unrelated
render slot, table roles are absent, and LayoutBuilder drops baseline.

### LayoutBuilder constraints, inherited updates and skipped-layout scope

[Seven upstream dispositions](upstream-layoutbuilder-dependencies-and-hidden-boundary.md)
bring complete pairs to 266/8,965 (208 match, 58 differ), with
134 partial and 38 reference-only. Normal inherited changes
are paired; Flutter's skipped-overlay cases remain reference-only.

### LayoutBuilder stateful resizing

[Five upstream dispositions](upstream-layoutbuilder-stateful-sizing.md)
bring complete pairs to 268/8,965 (210 match, 58 differ), with
135 partial and 40 reference-only. Resizing from either parent
or child and scaling inherited text behave as expected.

### SliverLayoutBuilder measured geometry and hit routing

[Five upstream dispositions](upstream-sliver-layoutbuilder-box-adapter.md)
bring complete pairs to 271/8,965 (213 match, 58 differ), with
137 partial and 40 reference-only. Actual transforms and
input routing match the checked original samples.

### Remediation checkpoint: 400 fully paired registrations

At the committed audit baseline (`e57497ce`), 400/8,965 original
Flutter test registrations (4.46%) are fully paired: 305 matched and
95 differed. There are also 202 partial and 80 reference-only
dispositions. The frozen frame series and authored probes are reported
separately. This is evidence coverage, not an estimate of fixes remaining.

Per the November 24 direction, implementation began at that threshold.
Committed fixes before this clipping patch: filtering byte offsets
(`51fbd720`), grapheme-limit rejection (`44adc2c9`), retained
IndexedStack painting (`7a1c5010`), and Positioned Stack geometry
(`6aaf14fd`). The [Stack evidence](upstream-stack.md) includes the
layout-only clipping repair and retained wgpu recheck. Remaining work
includes focus state for hidden, detached, and ancestor-blocked nodes,
directional defaults, other known mismatches, and the unaudited corpus.
Use a single focused build or test at a time and reuse the laptop cache.

### Continued input remediation and TapAndDrag coverage

The [behavior remediation](behavior-audit.md) verifies canceled mobile
touches, native formatter selection/composition policy, and rejection
of ineligible editors before switching the active IME client.
[Native IME actions](frame-physics-layout-editing.md) now distinguish
raw mobile Enter, explicit completion/traversal, and multiline newline
notification. The [single-tap overlap repair](upstream-tap-recognizers.md)
keeps the first contact through either release order. The next
[TapAndDrag comparison](upstream-tap-and-drag.md) adds 13 full matches
and one partial match: 413/8,965 originals fully compared (4.61%).

### Retained geometry and no-selection remediation

Fully paired coverage is 423/8,965 (4.72%) at `4d73ed14`:
328 matches and 95 historical comparison mismatches; 208 partial and
78 reference-only. The [FractionalOffset checks](upstream-fractional-offset.md)
added exact conversions and corrected Alignment's arithmetic origin.
[IndexedStack](upstream-indexed-stack.md) now handles explicit no-selection
without removing its children; six retained Flutter and Windows wgpu
frames agree across inside and edge probes. The separate authored layout
tests also verify selected and highest-child baseline forwarding.

### Pointer and scheduler remediation

Verified coverage reached 440 / 8,965 original registrations (4.91%) at
`ef2c82c0`: 344 matches and 96 historical comparison mismatches, 209
partial, 80 reference-only. The [Listener audit](upstream-listener.md)
checks bubbling, touch hover and transform-dependent local event fields.
The [behavior audit](behavior-audit.md) records the frozen pointer transform
and the same-frame callback and scheduler timestamp repairs.

### Rendering remediation and color effects

Fully paired coverage is 443 / 8,965 (4.94%) at `8976ce51`: 347
matches, 96 historical mismatches, 210 partial, and 80 reference-only.
The native [rounded clip regressions](upstream-tween-transform-and-lifecycle.md)
distinguish hard, ordinary antialiased, and isolated edges; the
transformed hard-clip probes cover both raw rounded rectangles and
paths. Default matrix ImageFiltered sampling now matches the pinned
edge samples. [Color filter checks](upstream-fit-overlays-scroll-notifications-clipboard.md)
cover matte bounds, nonseparable blends, saturation and the original
retained-layer identity case without counting authored samples as
extra original registrations.

The original childless ColorFiltered case raises full coverage to
444 / 8,965 (4.95%) at `b364dd8d`. Tab traversal from a directly
focused but traversal-excluded node is verified separately in the
[focus recheck](upstream-repeat-focus-restoration-flex-and-image-filter.md#focus-traversal-from-a-directly-focused-skipped-node).

The [clip default and zero-area recheck](upstream-tween-transform-and-lifecycle.md#default-canvas-rounded-coverage-and-widget-clip-originals)
adds three fully paired originals: 447 / 8,965 (4.99%),
351 matches and 96 historical mismatches. The separate Windows
wgpu readback verifies default rounded Canvas antialiasing under scale.

Two retained clip-mode cases (:107 and :123) bring the total to
449 / 8,965 (5.01%), with 353 matches and 96 historical mismatches.
The separate noncircular ClipOval readback also verifies ordinary
antialiasing without changing hard-edge or pointer routing controls.
The [elliptical save-layer recheck](upstream-tween-transform-and-lifecycle.md#elliptical-save-layer-compositing)
verifies that Windows wgpu now applies coverage once after the children
compose, keeping a smooth edge without leaking earlier paint.

The [default ClipPath recheck](upstream-clip-visibility-focus.md#default-and-childless-clip-path)
adds two original matches, for 451 / 8,965 fully compared (5.03%).
There are 355 matches and 96 historical mismatches. The default
outline tracks the laid-out child; childless modes retain the same
render object without emitting a paint clip. Native general path
coverage remains open.

[ClipPath.shape](upstream-clip-visibility-focus.md#shape-clip-direction-and-actual-layout-size)
adds the original inherited-direction and no-reclip comparison,
for 452 / 8,965 fully compared (5.04%), with 356 matches and 96
historical mismatches. In a separate matched probe, the outline
now follows the actual child rather than the maximum constraint.
The separate [fractional ClipRect recheck](upstream-clip-visibility-focus.md#fractional-rectangular-clip-modes)
verifies that the default stays hard and that ordinary antialiasing
and isolated antialiasing now have distinct Windows wgpu edge behavior.

[Childless and zero-size rect clips](upstream-clip-visibility-focus.md#childless-rect-clips-and-zero-sized-fitted-semantics)
add three matched originals, for 455 / 8,965 fully compared (5.08%):
359 matches and 96 historical mismatches. This includes clipping
a zero-size FittedBox with semantics enabled.

[Directional ClipRRect](upstream-clip-visibility-focus.md#directional-rounded-clips)
adds two matched originals: 457 / 8,965 fully compared (5.10%),
361 matches and 96 historical mismatches. It also fixes stale paint
when inherited values update retained compositor geometry.
The [rotated Canvas recheck](upstream-clip-visibility-focus.md#rotated-canvas-hard-rectangle)
verifies that the hard rectangle no longer leaks in the corners of
its axis-aligned bounding box on Windows wgpu.

[Custom clipper notifications](upstream-clip-visibility-focus.md#custom-clipper-notifications)
add one matched Flutter original, for 458 / 8,965 fully compared
(5.11%): 362 matches and 96 historical mismatches. A geometry change
can now reclip a mounted child without layout or picture recording.

[Optional numeric operands and dedicated gravity](upstream-physics-helper-comparisons.md#optional-operands-and-decimal-gravity)
add one original comparison: 459 / 8,965 (5.12%), 363 matches
and 96 historical mismatches. The same batch repairs the known
9.81 gravity drift without double-counting the earlier cases.

[Friction precision](upstream-physics-comparisons.md#precision-remediation)
remediates five existing pairs (three dedicated friction and two Newton
cases). The full focused animation/physics selection passes, including
both previously red strict friction tests. The original count stays
at 459 / 8,965.

[Simulation diagnostics](upstream-physics-comparisons.md#simulation-diagnostics-and-invalid-spring-duration)
add one original match: 460 / 8,965 fully compared (5.13%), with
364 matches and 96 historical mismatches. The same focused batch
backfills the debug guard for invalid spring durations.

[Ticker suspension](upstream-lifecycle-clipboard-comparisons.md#suspension-remediation)
repairs two historical mismatches: a paused ticker remains active
without asking for animation frames, and a ticker started while
paused first ticks after resume. Forced restoration frames can still
execute post-frame callbacks.
The [C04 recheck](focused-comparisons.md#c04-retained-recheck)
now defers start and restart from a transient callback until the
next frame under both runners. The original audit count is unchanged.

[ValueNotifier](upstream-foundation-scroll-comparisons.md#typed-value-notifier)
adds one matched foundation original: 461 / 8,965 fully compared (5.14%),
365 matches and 96 historical mismatches. Equal assignments do not
notify or rebuild a subscribed widget; unequal assignments do.

The [exact scheduler cancellation recheck](upstream-semantic-headings-clamping-and-dilation.md#exact-queued-callback-recheck)
moves the previous partial into the matched column: 462 / 8,965
original cases fully compared (5.15%), 366 matches and 96
historical mismatches. The same focused run verifies a dropped last
animation controller cancels its awaiting future instead of hanging.

[Heading levels](upstream-semantic-headings-clamping-and-dilation.md#heading-level-remediation)
complete one prior partial, for 463 / 8,965 original cases (5.16%),
367 matches and 96 historical mismatches. Invalid levels no longer
advertise a heading, and retained merged nodes inherit the child's
level only when the parent has none.

The [empty FittedBox recheck](upstream-semantic-headings-clamping-and-dilation.md#empty-fitted-content)
now removes both the child draw and its accessibility subtree when
the fitted area is empty, and restores them when area returns.

[FittedBox layout](upstream-fit-overlays-scroll-notifications-clipboard.md#fittedbox-layout-aspect-ratio)
adds two exact originals: 465 / 8,965 fully compared (5.19%),
369 matches and 96 historical mismatches. Independent constraints no
longer distort the fitting box's aspect ratio, and switching from and
back to scaleDown relayouts in place.
