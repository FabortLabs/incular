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

## Distinct value and status observations

Pinned TrainHoppingAnimation `:136` now reports [Forward] and then
[Dismissed], not two of each when both tween inputs share a controller.
The authored Proxy comparison also now matches: identical and equal
swaps notify neither channel, a status-only swap reports only status,
a value-only swap only value, and the detached parent cannot notify.
Observing does not add a reactive build dependency; regular reads still
track normally. Reentrant value changes report each distinct value once.
The following authored control separately pins the hop callback order.

### Hopping to a new train

Both frameworks deliver the new train's status first, then its value,
then onSwitchedTrain, even when the value observer was registered
first. The pending, uncrossed train does not replay an unchanged value;
the departed train stops notifying immediately. The reference explicitly
sets the initial controller value before registration to compare states
without its first unrelated status report.

## Raw menu controller and close lifecycle

Six pinned originals and their focused Incular counterparts now pair.
Replacing the controller for either an anchor or a group detaches the
old handle without changing whether the retained menu is open. Disposal
detaches both kinds as well. The detached handle reports closed, close
is safe, and closeChildren produces the expected debug attachment
diagnostic. Handles obtained from a descendant share that ownership.

The native close-order cases open the child using actual retained
pointer events. Both forms close children before their parent even if
a child defers requests. The authored reference also verifies that a
deferred parent's **request** reaches each child, but invoking the
parent's hide callback then forcibly closes children first.

Five separate legacy RawMenuAnchor fixture failures were reproduced
with the production patch reverted. They are not covered by this
lifecycle repair; focus adoption and outside interaction remain separate
checks.

### MediaQuery size and pending hide

Pinned `:3027` pairs with `flutter350_raw_menu_resize`: changing
only the ambient MediaQuery size requests no close while hidden,
exactly one close while open, and none after hiding. The native
test holds the physical viewport fixed and mutates a user signal
from the close callback, outside the read-only build phase. It
checks the open panel actually disappears from retained painting.

Pinned `:2602` and `:2705` pair with the native pending-hide test.
One attached controller close requests hiding without dismissing the
menu until the supplied callback is invoked; a saved callback invoked
after disposal is silent. No native timer or physical resize is
required for these three behaviors.
