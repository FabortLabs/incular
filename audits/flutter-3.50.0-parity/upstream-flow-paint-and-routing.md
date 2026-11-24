# Flow paint, repaint and overlapping input

Pinned Flutter 3.50.0 `ab2912035e`; unchanged Incular production `c647cb85`.

All six exact Flutter registrations passed. The original repeated
hit test (:64) starts at the same overlap and records [0,1,2,1]
on both. A change to the existing Flutter repaint Listenable
moves the overlapping click to child 0. Incular's equivalent
delegate state changes without invalidating its layout; the next
click still targets child 1. Replacing that actual delegate moves
the hit to child 0. Partial: Rust has no public Flow repaint
Listenable, so the test uses its existing retained delegate data
and proves the explicit replacement control.

Duplicate child painting reports the same offending index (:117)
via a Rust panic and typed error, not Dart's FlutterError string.
Requested alpha .2 appears in submitted Rust paint commands
(:142), not a comparison to Flutter's private nested layer types.
The zero-area Placeholder frame (:210) measures zero on both.
Default/custom clip properties on Flow and Flow.unwrapped
(:160, :185) passed only on Flutter and are not represented by a
public Rust clip setter or unwrapped constructor.

A separate pinned test changes requested paint order in the same
overlapping two-child layout. Flutter hits [1] and then [0] for
[0,1] and [1,0]. Rust hits [1] both times. This is deterministic
selection of the last child by index rather than the delegate's
last paint call, distinct from the repaint subscription gap.
See `flutter350_flow_upstream_parity` and
`flow_delegate_paint_order_test.dart`.
