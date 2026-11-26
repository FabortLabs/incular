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

### Retained menu integration controls

The eight older native controls are now green. Their fixture gives the
TapRegionSurface a hit-testable background and routes down/up input,
attaches real Focus widgets for explicit nodes, and advances the frame
scheduler. The previous five failures were reproducible with the prior
production code because the fixture had neither an outside hit target
nor attached focus nodes. No new pair is counted for the fixture repair.

### Nearest ancestor scroll activity

Pinned `:3076` and `flutter350_raw_menu_scrolling` agree: jumping
while closed does not request a close, a routed tap opens the panel,
and the out-of-range jump to 500 (400 maximum) starts a spring and
requests one close. The panel disappears while the anchor stays
mounted, and a later jump while closed does not call the handler.

The authored `raw_menu_scroll_activity_test.dart` and mounted Rust
control distinguish scrolling state from scroll notifications. An
idle in-range jump and a farther ancestor's animation keep the
menu open. A nearest-position animation requests a deferred hide
once at activity start, not on updates, and again on return to idle
if it is still open. The retained subscription does not consume
bubbled notifications and is replaced on a nearest-position change.
Mid-activity jump takeover and reduced-motion substitutions are not
covered by these comparisons.

### Scrolling the panel and listener teardown

Pinned `:2066`: an actual wheel scroll over the menu panel moves its
inner viewport but leaves both menus open, without firing a close
callback. An out-of-range scroll of the anchor's enclosing viewport
then closes both, reporting the submenu before the root. Pinned
`:2137`: a mounted root registers on its scroll position, and removal
unsubscribes. The native fixture also swaps positions while open,
verifying that only the current scroll position can close the menu.

### Delayed hiding and outside input

Pinned `:2632` and `:2669` keep the panel and withhold onClose at
the request and at 99 ms; the supplied hide removes the panel and
reports one close by 101 ms. The Rust fixture uses an application
timer queue and drives the supplied callback, not the controller's
direct visibility setter. The authored pinned Dart and native probes
also verify that a repeat request reaches a still-attached hidden
anchor but cannot report another actual close.

Pinned `:2751` explicitly expects close *requests* from parent
to deepest child despite the registration's title. Pinned `:2811`
checks the opposite order for each subtree's actual close after
scheduled callbacks: depth order [3,2] at 11 and 34 ms, then
[3,2,1,0] at 36 ms; no new callback at 56 ms. The native test opens
the three child menus via real pointer routes and checks the panel
and each retained controller as well as the event order.

Pinned `:2986` and the retained surface test both open via a real
anchor tap. One outside tap reaches the panel's TapRegion handler,
requests one close, and removes the visible overlay.

### Dismissal from a submenu

Pinned `:1393` and `:2921` now have focused runtime counterparts.
After three actual anchor taps and focus on the deepest button,
the public DismissMenuAction requests closure from the root. The
default path hides the entire overlay chain; deferred callbacks run
in root-to-leaf request order and leave the visible panels in place.

The authored pinned attachment case also matches. A mounted closed
submenu enables dismissal; hiding its root unmounts the submenu and
disables it. Native disposal revokes controller and scrolling access
at element unmount, even if the retained builder still has a cached
reference. The native command bridge rejects disabled invocations
without announcing an action invocation.

### Inherited menu state and anchor builders

Pinned `:600` and the native mounted scene observe the same group
panel and anchor values [false,true,false] and the overlay value
[true]. Opening a nested submenu does not rebuild any of those
observers when their nearest open state hasn't changed.

Pinned `:865`: maybeOf is an untracked controller lookup. Opening
does not rerun the panel or anchor lookup; an independent parent
rebuild reads the new controllers in panel, anchor, and overlay.
The overlay's mounting and parent-driven rebuild counts match.

Pinned `:2174`: tapping the custom anchor toggles its closed/open/
closed painting and calls onOpen and onClose once each. Its builder
runs under its own menu scope, so the lookup and explicit controller
argument resolve the same handle. The test positions the panel
below the anchor as the pinned original does.

### Escape routing and unrelated focus

Pinned `:1081` pairs: an actual Escape event from the group's
sibling focus target or the deeply nested panel closes the whole
menu tree. The authored Flutter probe and mounted native case also
agree that one Escape delivers one deferred request per member and
does not reach an enclosing handler when the focused menu is open.
Closed menus and focus unrelated to the open menu route to the
enclosing handler instead of dismissing every transient in the
window. The existing early-tooltip and focus-stage tests still pass.

The closed-anchor intent original (`:1147`) is partial: the native
test covers ordinary outward shortcut routing and Escape, not
direct dispatch of every Flutter focus-traversal intent type.

### Anchor and overlay geometry

Pinned `:1470` and `:1491` pair with the routed anchor tap, the
mounted position returned to its overlay builder, and the actual
200 by 200 panel at the anchor's top-left. The builder receives
the full 800 by 600 viewport. Pinned `:1051` and the additional
numeric reference agree on a scaled/translated 100 by 20 anchor
at (300,340) with size 120 by 24 (native f32 tolerance 0.0002).

Pinned `:3168` and the numeric control both measure the open
anchor moving upward by 100 pixels when a leading sibling is
removed. The retained menu stays open and the overlay builder
gets the new rectangle. Pinned `:3212` and `:3228` confirm a
zero-area root can open safely and a zero-area group remains closed
after its controller's no-op open call.

### Outside activation consumption and groups

Pinned `:1576`/`:1654` pair with real native pointer streams:
a closed menu does not consume the outside button, and an outside
tap closes the open tree descendant first regardless of consumption.
Only an open tree with consumption enabled prevents the outside
button's selection.

Pinned `:1522` pairs with a panel positioned away from its anchor:
the same anchor tap is inside while the panel joins the anchor's
TapRegion group, but outside after the reopened panel omits the
group ID. The test asserts mounting and both callback results.

### Delayed opening, position, and disposal

Pinned `:2401`, `:2432`, and `:2467` pair with precise request and
onOpen counts, null and explicit (10,15) positions, no panel before
show, and two open notifications when the supplied show runs twice.

Pinned `:2520`: after the application's 100 ms delay, the native
retained panel appears at (0,0), then moves to (50,50) only after the
second request's delay. Pinned `:2563`: calling the saved show after
the anchor has unmounted neither paints a panel nor fires onOpen.

### Nested overlay selection

Pinned originals `:2227` and `:2297` pass on Flutter. The native
menu previously chose an overlay independently for each submenu:
opposite settings produced a detached child surface in the retained
fixture. Now submenus inherit the root menu's choice, including
retargeting an already open submenu when the root choice changes.
The retained surface tests verify both opposite settings and the
live update.

These remain partial. The Flutter originals inspect the actual
nearest versus outer Overlay theater when two Overlay hosts are
present. Incular's layout portal still uses the owning view as its
layout coordinate space, and the retained parent-surface observation
does not prove separate Overlay hosts, clipping, or relative bounds.

### Group state and focus after closeChildren

Pinned `:186` closes the submenu popup, not the parent or the
focused submenu anchor. The native runtime opens each by pointer,
then verifies its explicit focus node remains primary after closing
only the children.

Pinned `:311`, `:362`, `:397`, and `:435` pair with the
retained group: the group tracks whether either descendant is open,
including one behind Padding; tapping switches the visible panel
and retapping closes it. Group open does nothing, while both close
methods clear the appropriate popup and leave its anchor mounted.

#### Painted and interactive controls under a transformed anchor

The additional pinned geometry probe positions a 200 by 40 control
80 pixels right and 65 down from the transformed anchor's origin.
Flutter paints/interacts at (380,405); the native baseline instead
reported (46,128) with size 240 by 48. The native fix keeps the popup
out of the anchor's size calculation and cancels the anchor's
invertible scale/rotation at the lifted overlay boundary. Normal
translations keep their faster offset path.

The native test now compares published anchor geometry, the actual
retained and painted control rectangles, and a routed tap near the
desired far edge. Focused menu, tooltip, existing OverlayPortal,
transient-surface, and layout-builder neighbors passed. No new
upstream registration is counted for the authored probe. Singular
anchor transforms and separate Overlay host coordinate spaces are
not covered by this comparison.

### Nested visibility and context-menu anchors

Pinned `:82` toggles a live panel on actual anchor taps. `:108` opens
and closes the nested and parent controllers in both orders, with
the panel markers and attachment state following both closures.
`:234` and `:559` use real taps to switch the visible menu between
sibling anchors, under either a parent menu or a group. In `:279`,
the second context menu is part of the first menu's anchor builder,
not its panel; tapping each visible anchor still opens its panel.

### Group resize does not write during build

Pinned `:2366` resizes an open child under a menu group without a
setState-during-build exception. The native runtime changes both the
frame constraints and the inherited MediaQuery, and checks that
onClose can mutate an application signal without hitting the pure
build guard. The group and child both close and the next scene no
longer contains the panel.

### Controller moves and open/close notifications

Pinned `:701` swaps both controllers without disturbing the visible
panel; only the new pair reports open and can close it. Pinned `:759`
and `:811` move the same handle onto a newly keyed group or anchor.
The old open state is discarded, but the retained replacement accepts
a real anchor tap and is closed by the original handle. Pinned
`:1734` and `:1766` verify the exact open/close callback sequence
from both anchor taps and programmatic calls, without replaying
onOpen during close or onClose during open.

### Focus traversal keeps application shortcuts

Pinned `:1215` and `:1881` send actual key events from focused
anchor and panel controls. The custom traversal actions get the same
left/next/previous sequences (and the grouped next/previous sequence
both before and after opening). The original node stays primary.
Home and End don't cause built-in raw menu focus movement.

### Group panel geometry and default root closure

Pinned `:1445` pairs with an unstyled menu group wrapping a padded
row: two 100 by 100 children and 8 px all around yield (0,0,216,116).
Pinned `:1981` and `:2023` pair with default close behavior, not a
custom close-request handler. A view resize or a jump beyond an
ancestor's zero scroll extent produces the onClose notification once
without replaying onOpen, and the overlay visibly disappears.

### Programmatic inherited actions

Pinned `:1147` now pairs using actual scoped invocation rather than
only key events. A focused closed anchor forwards directional up,
next, previous, and dismiss commands to the application in order.
When the same menu is open its nearest dismissal handles the intent.

Pinned `:1340` pairs using two different callback payloads sent to
the same inherited action from the mounted anchor and panel contexts.
Each callback executes once, and neither invocation closes the menu.
The Rust typed command selects the registry entry by identity and
preserves each invocation's callback through type erasure.
