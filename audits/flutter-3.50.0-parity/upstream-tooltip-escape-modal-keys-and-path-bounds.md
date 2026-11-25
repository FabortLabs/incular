# Escape handling, modal barriers, keyboard listeners, and path bounds

Pinned Flutter 3.50.0 `ab2912035e`; unchanged Incular production `c647cb85`.

- All three RawTooltip Escape originals pass on Flutter. Rust
  leaves a hovered single tooltip (:1186) and both programmatically
  opened tooltips (:1241) mounted after Escape. Explicit hiding
  closes them as a reachability control. A scoped Rust ancestor
  shortcut fires on both presses while the tooltip stays open
  (:3290), whereas Flutter closes the tooltip on the first without
  invoking the ancestor, and invokes it on the second. The third
  case is partial because it uses Incular's named shortcut action,
  not Flutter's typed DismissIntent. Rust's default tooltip
  transient policy opts out of Escape and its portal has no
  dismissal callback. See `flutter350_raw_tooltip_escape_parity`.
- Flutter and Incular block taps behind AnimatedModalBarrier and
  allow those in front (:510, :556). With a real mounted route,
  primary release pops, but secondary release leaves Incular's
  modal in place (:665, :696). In the normalized test both buttons
  are Touch samples to match Flutter's tester default. These route
  comparisons are partial (no-transition RouteOutlet rather than
  Flutter's named route), not mouse or third-button claims.
  See `flutter_modal_barrier_upstream_parity`.
- KeyboardListener no-key removal (:10) matches completely.
  Left Meta Down/Up (:42) and removal stopping future delivery
  (:66) match normalized Rust inputs rather than the browser/
  Fuchsia source adapters. See `flutter350_keyboard_listener_parity`.
- A separate authored Path API check finds Rust returns tight
  curve-extrema bounds where Flutter includes Bézier control
  points. Two closed paths from (10,10) have equal width 40,
  and Flutter quadratic/cubic heights 40/60 vs Incular 20/45.
  Rectangle, translation, and containment controls agree. This
  is a public bounds difference, not erroneous rendered or hit
  pixels and not necessarily wrong for internal culling.
  See `flutter350_path_curve_bounds` and
  `path_curve_bounds_test.dart`.

## Public Bézier bounds repair

The same pinned quadratic and cubic probes now agree on public heights
40 and 60. Actual geometry bounds remain 20 and 45, and both containment
samples still agree; the public bounds change does not expand the
painted shape. The immutable geometry shares a lazily computed public
control box, while shape sizing, culling and damage use its cached tight
geometry box. A same-geometry path keeps its resource identity. Twelve
focused rendering tests, 48 widget paint/layout tests, two wgpu
tessellation tests (no GPU device required) and both Flutter probes
passed. This closes an authored probe, not an additional upstream
registration site.

## Current RawTooltip Escape routing

The three original Escape registrations (:1186, :1241, :3290) passed on the
pinned Flutter checkout and all three mounted Incular tests now pass.
One Escape closes a hovered tooltip without mouse exit or closes both
programmatically opened tooltips. When an ancestor has an Escape dismissal
shortcut, the first press closes the tooltip without invoking that shortcut;
the next press invokes it. The early stage only dismisses tooltip portals;
other transient roles keep their ordinary key routing. The shortcut case is
still partial because Incular's named action is not Flutter's typed intent.
