# Focused frame rechecks, batches two and three

Two more bounded batches passed the reference runner and compared actual
frame traces. Audit baseline is still Flutter 3.50.0 `ab2912035e` and
Incular implementation `c647cb85`. Totals including the
[first batch](frame-rechecks.md): **20/253 existing frame fixtures (7.91%)**,
19 matches and one mismatch. The repeated fade-pop case below does not
increase the number of distinct fixtures.

| Scenario | Result | Frames |
| --- | --- | ---: |
| gestures__tap | match | 3 |
| gestures__double_tap | match | 9 |
| gestures__vertical_drag_down | match | 7 |
| layout__stack__positioned | match | 2 |
| semantics__action_dispatch | match | 2 |
| physics__tolerance__default | match | 4 |
| physics__spring__damping_regimes | match | 41 |
| physics__bounded_friction__distance_tolerance | match | 3 |
| anim_core__ticker_mute_resume | match | 9 |
| navigation__navigator__fade_pop_timeline | repeated mismatch, not a new disposition | 47 |

These ordinary gesture, semantics and physics controls do not clear the
different composed/timing/precision cases in the custom reports. No stale
reference traces were reused.

## Runner

From the Incular checkout, on the Windows audit machine with the pinned SDK
bootstrapped and its test dependencies cached:

```powershell
& ./audits/flutter-3.50.0-parity/run-focused-frames.ps1 -Scenario @(
    'gestures__tap',
    'physics__tolerance__default'
)
```

The script limits a batch to 1–5 exact catalogued names with no substring
ambiguity; it does not run the full harness or the workspace suite. It
runs the selected Flutter recordings and Rust frames in strict mode and
writes fresh logs, reports and `results.json` under the ignored
`.tmp/flutter350-full-audit/frame-batches/` directory. It returns nonzero
on a mismatch or missing/failed result. Validated with both matching
fixtures and a repeat of the fade-pop divergence.

On Linux, use the same exact bounded scenario selection:

```sh
bash audits/flutter-3.50.0-parity/run-focused-frames-linux.sh \
  scroll__drag__notification_order scroll__fling__notification_order
```

This variant runs with the pinned SDK and cached dependencies, isolates
the reference package in scratch, and serializes the native build and
test. It leaves the source harness manifest unchanged.
