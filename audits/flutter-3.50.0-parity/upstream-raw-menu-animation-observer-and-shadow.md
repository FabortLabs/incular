# Raw menu closure, animation notifications, and shadow interpolation

Pinned Flutter 3.50.0 `ab2912035e`; unchanged Incular production `c647cb85`.

- All five exact RawMenuAnchor originals passed. A detached
  controller safely closes and reports not open (:953, :962).
  Group closure notifies child before root (:513). With a deferring
  child close handler, Flutter still closes child and notifies it
  first (:474). In Incular the root calls onClose, but child stays
  logically open and never calls onClose. The menu tree tests open
  the mounted submenu programmatically instead of tapping an anchor,
  so those two are partial. Replacing a controller (:969) leaves
  the old Incular controller able to open the same anchor as the
  replacement and allows its closeChildren call. Flutter detaches
  the old controller; the original debug closeChildren assertion
  passes. Only Rust tests also assert old open authority and a live
  replacement control. See `flutter350_raw_menu_close_parity`.
- All five animation graph registrations passed on Flutter.
  Parent assignment/updates match (:60). Default, removal, and
  switching active train (:51, :77, :97) match outputs, not
  Dart-only diagnostics. Train status (:136) fires Forward and
  Dismissed twice in Rust with both trains driven by the same
  controller, instead of once. In a separate verified authored
  ProxyAnimation comparison, equal/identical swaps and status-only
  or value-only updates spuriously notify both Rust channels;
  snapshot values, active status, and detachment of the previous
  parent match. See `flutter350_upstream_animation_observers`
  and `proxy_parent_swap_reference_test.dart`.
- Two original BoxShadow scale/list and BlurStyle controls
  (:101, :138) match scoped numerical/style properties (not
  original hash/identity/diagnostics). A separate pinned
  `box_shadow_extrapolation_test.dart` finds geometry extrapolates
  on both; only Rust clamps color at negative and >1 fractions.
  Flutter color [32,32,160,255] vs Rust endpoint [64,64,128,255]
  at -0.5; a symmetric divergence occurs at 1.5. Midpoint
  matches. See `flutter350_box_shadow_interpolation`.

### Current shadow color extrapolation

The authored pinned Flutter and native BoxShadow samples now agree at
-0.5, 0.5 and 1.5 for color, offset, blur and spread. The shared color
helper accepts an unclamped factor for shadows and saturates only the
output channels. Generic bounded color interpolation stays unchanged.
