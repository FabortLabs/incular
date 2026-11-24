# Fit, overlays, scrolling, notifications, clipboard, and color effects

Pinned Flutter 3.50.0 `ab2912035e`; Incular production `c647cb85`.

- FittedBox contain, unclipped cover paint, and transformed pointer
  delivery match (:41, :71, :396). Updating the same mounted
  FittedBox from ScaleDown to Contain and back gives heights
  [50,100,50] in Flutter and [50,50,50] in Rust (:487).
  The Rust layout branch does not use fit. This related C32
  regression tests a retained update with finite width and loose
  height. No claim about Rust's absent public clipBehavior option.
  See `flutter_fitted_box_geometry_parity`.
- FractionallySizedBox nested fractions and absolute top-right
  alignment under RTL match (:9, :32). The zero-size host case
  (:100) matches under equivalent layout constraints, not actual
  native viewport resize. Tight OverflowBox (:10) matches;
  it does not resolve C31's loose maximum constraint gap.
  See `flutter_fractional_overflow_geometry_parity`.
- OverlayPortal show/hide (:765) and inherited declaration-local
  directionality (:104) match. First builder evaluation and the
  pointer hit in moved overlay (:810, :893) match with an explicit
  Rust Builder and zero-margin anchored mount; these are partial.
  Different controller, root-overlay, and listener-order APIs
  are not implied. See `flutter_overlay_portal_upstream_parity`.
- All five exact scrolling physics behaviors match: unchanged
  initial release velocity, increasing resistance, easier easing,
  fast physics's zero easing resistance, and independence from
  content length. The gesture/scroll coordinate sign is normalized.
  See `flutter350_upstream_overscroll_offsets`.
- Manual custom notification dispatch and cancellation (:15, :43)
  match synchronous bubbling, same payload identity, and inner
  listener consumption. Flutter subtype matching is not equivalent
  to Rust's exact-type handlers and was not counted. See
  `flutter_notification_upstream_parity`.
- EditableText's clipboard gating matches the original neutral
  subsets for empty clipboard, read-only and obscured states
  (:2788, :3118, :3152), not their toolbar appearance. Read-only
  obscured Select All (:3235) disagrees on selection. The original
  uses programmatic selection; the independently verified
  `select_all_read_only_obscured_reference_test.dart` also compares
  the actual same Ctrl+A path with a non-obscured positive control.
  Flutter leaves the caret collapsed, Incular selects all 0..9.
  See `flutter350_clipboard_menu_gating_parity`.
- Original ColorFiltered sepia and zero area report passes
  (:33, :119). The custom pair tests other matrix values and
  actual presented zero-area pixels, not original golden
  equivalence. Opacity order and all paints inside the child
  match. A matrix that makes transparent black opaque should
  produce [0,32,0,255] through the enclosing clip; Windows wgpu
  only filters the child's own bounds. This is one scoped
  bounds discrepancy, not separate defects per nesting case.
  See `flutter350_color_filter_alpha_order_raster` and
  `color_filter_alpha_order_raster_test.dart`.
