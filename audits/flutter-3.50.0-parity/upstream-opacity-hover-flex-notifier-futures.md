# Opacity, hover, spacing, notifier dispatch, and ticker futures

Pinned Flutter 3.50.0 `ab2912035e`; Incular production `c647cb85`.

- Three original Opacity registrations report passes. The label
  retention/exclusion sequence (:19) matches in
  `flutter350_opacity_child_work`; original paint matcher and full
  semantics metadata were not ported. The original padded offset
  (:152) and empty layer (:179) have focused Rust equivalents in
  `flutter350_opacity_raster`, but are partial: not the original
  full image or Flutter's private layer graph. We did not verify
  availability or actual comparison of the original offline golden.
  The independent paired `opacity_raster_test.dart` and real Windows
  wgpu readback compare full 100×80 masks: initial and 17px-scroll
  stripe geometry, a presented black empty frame, and once-only
  group opacity across overlap (blue 128 vs 192 on separate groups,
  tolerance one byte).
- A separate authored test compares first child paint work.
  Flutter does not invoke the user painter at 0 with either
  semantic flag, or at .001 (rounded effective alpha zero). Incular
  invokes it once at all these values. The visible 1, .1, and .003
  controls each invoke it once on both. This concerns recording
  work/callbacks, not leaked pixels or measured frame-time speed.
- Four original MouseRegion cases passed. Unmount suppresses exit
  (:366) and a retained region uses the newly supplied handlers
  (:699). A stationary pointer sees correctly timed enter/exit and
  correct global positions, but local payloads wrongly equal the
  global position in Incular (:393, :433). Flutter uses the new
  transform for enter and the old transform for exit; Rust's frame
  refresh starts with local equal to global and the synthesized
  callbacks receive that value unchanged. See
  `flutter_mouse_region_lifecycle_parity`.
- All three original Spacer registrations (:9, :24, :60) and RTL
  tight Flex (:744) match geometry: proportional 1/1/2/4 allocation,
  ordinary and unbounded cross-axis columns, and reverse x offsets
  700/600/500 at y=250. See `flutter_spacer_flex_geometry_parity`.
- ChangeNotifier callback mutation (:168) and duplicate presence
  (:457) match. The error-dispatch part of :83 is partial:
  first-listener failure lets Flutter deliver to later listeners;
  a Rust panic exits dispatch. This is not a claim about identical
  FlutterError reporting or ownership/dispose. See
  `flutter350_notifier_dispatch_parity`.
- Both original late orCancel cases (:153, :167) match completion
  and explicit stop. The lifecycle case (:122) is partial because
  Flutter explicitly disposes an AnimationController, for which
  the closest Rust counterpart is dropping the last owned handle.
  Rust's associated future remains Pending instead of reporting
  cancellation across the next bounded frame/poll. See
  `flutter350_upstream_ticker_futures`.
## Last controller owner cancels its future

The pinned line :122 original still passes. After releasing the last
controller, the Rust continuation now catches TickerCanceled and reaches
its end instead of waiting forever. The controller owns a distinct future
from its optional scheduler ticker. A control verifies that dropping
one of two handles does not cancel a non-vsync controller; dropping
the last one does. The authored lifecycle and callback controls remain
green. This recheck keeps the original's explicit Flutter dispose vs.
Rust RAII distinction in the ledger.

## Observer failure isolation

The error-dispatch part of the original `foundation/change_notifier_test.dart:83`
was reproduced: a failing first listener previously cut off all later
listeners in that batch. Rust now reaches the same remaining callbacks,
preserves typed panic payloads and their order, and keeps the existing
snapshot behavior for adds/removes. `notify_collecting_panics` lets
the caller report all failures without rethrowing them; plain `notify`
resumes the first failure after completing delivery. The original stays
partial because Flutter's normal path reports to its global error
reporter rather than unwinding Rust application code.

The exact :83, :168, and :457 originals and 21 focused Rust checks
passed. Panic-abort builds cannot recover from an observer panic.

## Awaited controller chains under uneven frame times

Pinned `animation/futures_test.dart:12` and `:67` now have full
Runtime comparisons. Three real controllers run for 100, 600 and
300 milliseconds. Direct awaits and cancel-aware awaits each produce
the exact sequence at all ten original frame timestamps, including
duplicate zero-time frames and the missed completion deadlines.
The local continuation polls only after its ticker wakes it, still
inside that frame's timestamp. No sleeping or time tolerance is used.

:122 now records a partial match after the cancellation recheck,
keeping the explicit disposal versus last-owner release distinction.
:185 is partial: a completed ticker drives a normal generic Rust Future
continuation, but the Dart-only asStream, timeout, and promise methods
aren't reproduced. All four selected originals and the seven focused
Rust ticker-future tests passed. The adjacent run-semantics precision discrepancy was subsequently
[repaired](upstream-animated-size-overflow-controller-and-navigation.md#double-precision-linear-controller-values).


## Notifier disposal, diagnostics, and composition

Four pinned ChangeNotifier originals passed (:64, :83, :415, :480).
The disposal test pairs the observable debug assertion at the reporter
boundary: disposing from the callback reports one failure, never
returns to the following statement in that callback, and still allows
ordinary disposal after dispatch. The new Rust collector returns the
original payloads in notification order for an embedder's reporter; the
ordinary Rust entry point still resumes the first failure. The broader
:83 case stays partial because it uses Flutter's automatic global error
reporting and removal by callback identity rather than Rust ownership.

The merged-listenable diagnostic now preserves empty, absent, ordered
and repeated source slots without moving absent entries into dispatch.
It prints the Rust source type, not a Dart subclass instance name, so
:415 is partial. The Rust composition control matches the notification
and owner's state change in :480, but doesn't emulate Dart inheritance
or mixin syntax. All 26 focused Rust notifier, observer and allocation
tests passed.
