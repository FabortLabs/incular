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
