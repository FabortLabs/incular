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

### Modal barrier contact and release

Pinned :665 and :696 now pair with actual anchor input opening a
named, animated route containing an AnimatedModalBarrier. At the
settled frame, both primary and secondary press leave the barrier
visible and the modal current; release and reverse-animation
settling return to the initial route and remove the barrier.

The authored pinned Flutter probe and native retained test also
verify tertiary, back, forward, and secondary+tertiary masks.
The barrier uses raw arena tap recognition rather than a
primary-button GestureDetector. No semantic activation is added
by that recognizer. Five focused runtime cases and four existing
raw-arena cases passed. This comparison is normalized pointer
input, not OS-level mouse button integration.

### Barrier competition and pop vetoes

Pinned :732 and the authored competition control now match the mounted
route. A translucent foreground drag recognizer and the barrier tap
enter the same arena. A touch drag, a canceled touch, and a moving
secondary mouse contact do not dismiss; a stable tap dismisses
exactly once. The initial native any-tap repair consumed the stream
in the raw route before the foreground recognizer could see it.
The shared ordinary arena preserves hit order and the target's
child-before-parent chain.

Pinned :762/:808 and the native guard fixture call the pop guard
once on release and respect both results. The reference guards
resolve immediately; native uses its synchronous boolean decision
and compares settled route/marker presence, not future scheduling.
Pinned :854/:883 also pair: the supplied onDismiss runs once
instead of popping the route. Eight modal tests and focused
pointer, raw-arena, transformed-gesture and menu controls passed.

### Barrier semantics, hover and cursor

Pinned :525 and :600 and the mounted native Stack agree: an animated
locked barrier hides a MouseRegion behind it from hover, but the
MouseRegion in front sees both enter and exit. At :573 a translucent
20 px foreground drag reaches its recognizer even when the barrier is
locked. The pinned and native zero-area builds at :954 both lay out
without failing. For :1009 the actual runtime cursor remains Basic
over an underlying Click region; the native constant-color animated
path covers both locked and dismissible barriers.

Pinned :910 and native agree that a locked unlabeled barrier does not
add a semantic node. The three target variants of :922 passed upstream.
Incular now exposes separately dispatchable Activate and Dismiss actions
on the full-screen labeled veil. An explicit Semantics.on_dismiss is not
an Activate alias. The mounted test also disables semantic dismissal
and checks that stale actions no longer invoke the callback. This is
partial parity: native label retention and platform eligibility differ,
and AccessKit projects Activate to Click but has no separate Dismiss
action to project.

Pinned :630 passed: a tap on a locked barrier submits the SystemSound
alert service request. Incular still blocks that tap but has no paired
alert service behavior. This upstream test observes the request, not
whether a host audibly plays it.

### Rebuilding during a barrier contact

The authored Flutter probe replaces onDismiss between press and release
for primary and secondary contacts. There is no callback during the
replacement frame and release calls only the new handler. Incular
previously called the old handler. A retained arena member now preserves
the press-time button mask and competition but resolves its any-button
tap callback from the mounted widget at delivery. The new regression and
the earlier modal drag/cancel, pointer, and menu outside-tap tests pass.
This authored control is not counted as an upstream registration.

### Reactive accessibility bounds

Pinned :968 passed for iOS, macOS and Android. The native notifier
insets the local accessibility rectangle on each side before applying
world transforms. The exact reference vector (10, 20, 30, 40) on an
800×600 veil gives edges (10, 20, 770, 560) with Activate and Dismiss.
The pinned authored Android probe also verifies live replacement,
restoring zero insets and pointer delivery outside the semantic rect.
The native retained probe checks semantic hit edges, same semantic
node ID, identical paint commands, and zero rebuild, layout or repaint
work on notifier changes. It also verifies detachment when the notifier
is replaced, excluded or unmounted.

Incular does not emulate Flutter's platform gating; the geometry
applies whenever Incular exposes the barrier dismissal node.

### Non-animated ModalBarrier

The public `incular::widgets::ModalBarrier` is the static veil; the
main prelude exports it as `ModalBarrierWidget` to preserve the existing
navigation route configuration named `ModalBarrier`. Static and animated
descriptors share input and semantics; the static one is transparent by
default and has no animation ticker. A supplied color produces one
edge-to-edge solid fill without a border.

Pinned :55/:67/:91/:105/:132 and the native static veils cover rear and
foreground taps and hover plus a 20 px translucent foreground drag.
Pinned :191/:222/:258/:288/:334 and the mounted outlet agree on press
versus release, drag competition, and honoring both guard results.
The tested native guards decide synchronously; the pinned WillPopScope
callbacks resolve immediately. No fixture handler stands in for the
native static default. A nested-outlet test proves it addresses only
the nearest outlet and cannot pop that outlet's sole route or fall
through to an enclosing outlet. Pinned :380/:429 pair with explicit
callbacks, which bypass the default pop.

Pinned :456/:499 pair for a locked unlabeled barrier and for zero-area
layout. :1009 now exercises the static public type over a Click region.
:968 pairs with its exact static type, inset rectangle, label and
framework tap/dismiss actions. The native authored probe continues to
verify semantics-only invalidation. :466 remains partial: both explicit
actions, typed direction and full-screen bounds match, but native
platform gating and a separate AccessKit Dismiss projection do not.

Pinned :159 and :409 passed upstream but remain open in Incular: the
former requires forwarding the native system-alert request and the
latter observes Flutter's gesture error-reporting context.
