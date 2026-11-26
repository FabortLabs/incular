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

### Delegate order controls paint and hits

The pinned two-child authored comparison now matches. With overlapping
children, delegate calls [0,1] produce that draw order and hit [1];
calls [1,0] produce reversed draw order and hit [0]. Native geometry
and semantics keep their retained child identities, but the composed
and hit-tested order is the one actually painted; omitted children are
not attached to the paint itinerary. The original repaint-provider
registration (:64) is still open at this checkpoint.

### Repaint without build or layout

Pinned :64 now matches using the same unbounded AnimationController
as a repaint Listenable. The first hit trace is [0,1,2,1]; setting
the live controller to 50 changes the same overlap from child 1 to
child 0. The retained tree does not rebuild or remeasure any child.

An authored bounded controller on the pinned reference and native
runtime also reports paints at [0,0,0.3] with unchanged layout
counts and the moved hit target. The initial zero-value tick does
repaint in Flutter. Native render subscriptions wake the runtime,
detach on replacement, and tick the same Flow render without adding
a ticker widget around its child tree. Delegate painting and the
last paint order are now committed in paint, not during layout.

### Wrapped and unwrapped clip coverage

Pinned :160 and :185 pair with the default HardEdge setting and same
retained Flow render across Clip.none, HardEdge, AntiAlias and
AntiAliasWithSaveLayer. Native tests also read the actual viewport
draw clip and the isolated save layer, use overflowing translated
content, and confirm these updates do not remeasure the delegate.
The regular constructor adds child repaint boundaries; unwrapped
leaves them out. The existing Flow and repaint tests still pass.
