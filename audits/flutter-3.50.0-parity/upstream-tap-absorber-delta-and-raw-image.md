# TapRegion, AbsorbPointer, editing deltas, and RawImage

Pinned Flutter 3.50.0 `ab2912035e`; Incular production `c647cb85`.

## Pointer and semantic routing

- Eight exact Flutter TapRegion originals passed. The normalized
  primary mouse matches outside/inside mouse-up (:113, :339),
  suppression with consumption (:179), and updating a mounted
  region's group (:487). The focused Rust Runtime target is
  `flutter_tap_region_upstream_parity`.
- Outside/inside down callbacks on both right and middle mouse
  (:578, :652, :904, :978) do not fire in Incular. The same mounted
  raw listeners receive each button phase with the correct held and
  changed bits, and the primary control passes. The shared early
  filter precedes classification in `tree/raw_input.rs:736–747`.
  `flutter_tap_region_mouse_buttons_parity` checks callback membership
  per contact, not same-phase ordering, pointer add/remove,
  navigation scope, or secondary consumption.
- All three original AbsorbPointer registrations passed. Incular
  preserves delivery to a disjoint sibling (:11). The semantic
  tests (:26, :50) compare Flutter's test-local button to Incular's
  neutral retained action surface, so are partial rather than
  tests of identical focus adapters. With absorption off, the
  label, role, enabled/focusable flags, Activate and Focus match.
  With it on, the flags still match and Activate is suppressed,
  but only Incular advertises Focus. This is a tree difference,
  not a native screen reader or keyboard-focus claim. See
  `flutter350_absorb_pointer_upstream_parity`.

## TapRegion callback order

The separate painted-mouse callback-order probe
`tap_region_callback_order_test.dart` confirms outside callbacks run
before inside callbacks, on both down and up. Incular initially sent
inside first in both phases. The retained dispatch now matches
`A.outsideDown, B.insideDown, A.outsideUp, B.insideUp`. All eight
focused primary and navigation Runtime cases pass. This is an authored
ordering check, not a new original registration or a claim about order
between multiple callbacks in the same classification.

## Right and middle mouse buttons

All six pinned right/middle registrations (:578, :652, :726, :815,
:904, :978) and their mounted Runtime counterparts pass after removing
the primary-only filter. Before the fix, Incular reported none of the
right/middle classifications even though the raw pointer events arrived.
The four earlier mismatches and two newly reproduced consumptive
registrations are preserved in the evidence ledger.

The authored paired control verifies that, with consumption off, a
matching secondary or tertiary recognizer fires exactly once. With
consumption on it does not fire, and neither button becomes a primary
tap. A release with no buttons held clears the consumed raw stream;
the next contact on the same pointer can reach a recognizer again.
This slice does not compare native OS context-menu routing or button
chords.

## Primary down, cancel, and empty hit regions

The two primary down registrations (:39, :265) match at all five
positions, including outside the surface, and Cancel does not emit
another notification. In :405 the earlier Rust collector treated an
empty child's layout bounds as a hit. Flutter requires an accepted hit,
not merely a translucent entry in the path. The collector now tracks
that distinction separately for TapRegion delivery. An empty defer
target and an empty translucent target do not report inside; an empty
opaque target does and notifies its whole group.

All 31 focused pointer, semantics, navigation, hover and raw-arena
tests passed after the change. All 19 original registrations in
`widgets/tap_region_test.dart` now have paired dispositions. The
ledger preserves the verified historical mismatches; the family
checks do not cover native context menus or arbitrary pointer chords.

## Editing and raster defaults

- Three original TextEditingDelta checks match the observable
  applied text, caret and composing payload for deletion (:138),
  shorter replacement (:235) and selection/composition-only
  update (:336). Incular cannot represent Flutter's separate
  stored oldText/JSON variants, so these are partial.
  The independently adapted invalid insertion (:68) also exposes
  a policy difference: Flutter debug rejects the mismatch, while
  Incular clamps the out-of-range offset against its input and
  inserts a character. It is not a release assertion comparison or
  a wire-format test. See `flutter350_services_text_delta_parity`.
- A separate authored pair, not a static upstream count, measures
  RawImage on actual Windows wgpu. With omitted fit and a center
  slice, Flutter fills 100×60, Incular paints only a centered
  30×30 image. Explicit Fill, explicit ScaleDown, and no-slice
  controls match. Flutter defers the missing fit to paintImage;
  Incular hardcodes ScaleDown before the correct slice-aware
  fallback. See `flutter350_raw_image_nine_slice` and
  `raw_image_nine_slice_test.dart`.

## Reject malformed replacement ranges

The exact `services/text_editing_delta_test.dart:68` registration passed.
Its Rust reproduction previously turned an insertion at byte 11 in empty
text into an insertion at byte zero. The checked delta path now returns
a typed range error and the convenience path leaves the original value
unchanged. Reversed ranges, out-of-bounds offsets and offsets inside a
UTF-8 scalar do not reach controller formatters; a valid subsequent edit
still does. Thirty focused Rust tests passed across editing, delta/state,
and the runtime platform text-input bridge. This stays partial because
Rust does not store Flutter's separate oldText payload or use its debug
assertion.

## Blocking user actions without removing focus

Pinned AbsorbPointer originals :26 and :50 and the native retained
syntax now match the observable label, button role, enabled and
focusable flags, and Tap/Focus action mask. The comparison also uses
an explicit Focus node and Semantics annotation so the result is not
specific to Incular's neutral button surface.

Four authored checks extend the same three-frame transition to
AbsorbPointer, IgnorePointer, and Visibility retaining semantics
but not interactivity, plus a mixed merged node. The label, role,
semantics ID, and actual keyboard focus remain. The blocked state
omits focus, tap, long press, increment, decrement, directional scroll,
and custom actions, then restores them when unblocked. The mixed merge
keeps both labels without taking the blocked descendant's custom callback.
Flutter's accessibility focus lifecycle callbacks remain published;
Incular's portable action enum does not model those callbacks.

The native dispatcher also rejects a stale platform user-action request
before notifying TapRegion or invoking its callback. This is independent
of invoking Flutter's underlying callback directly. All 31 focused
runtime tests and 47 focused widget tests passed.

## Resolved RawImage fit default

RawImage now preserves an omitted fit until image painting, like
Flutter. Without a slice, the original centered 30×30 behavior stays
the same. With a slice, omitted fit paints all 100×60, as does explicit
Fill; explicit ScaleDown still paints centered 30×30. The Linux native
test verifies that all nine emitted patches stay inside those exact
bounds, fill the expected area, and do not overlap. It reproduced the
900-versus-6,000 pixel-area regression before the fix, then passed
with 39 adjacent native cases. The pinned Flutter authored test passed
four actual readback controls. The Windows wgpu reproduction remains
in the repository but was not re-run on the cloud computer. This is
an authored case, not an added upstream registration.
