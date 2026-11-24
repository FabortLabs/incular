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
