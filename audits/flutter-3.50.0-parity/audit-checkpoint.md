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
