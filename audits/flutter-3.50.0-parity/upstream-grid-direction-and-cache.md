# Grid direction and builder lifecycle

Four selected registrations in pinned `widgets/grid_view_test.dart`
passed on Flutter. Two retained Rust comparisons check all four
rectangle edges with a four-lane vertical grid changing LTR → RTL →
LTR. Without spacing, the first tile moves from (0,0) to (600,0)
with size 200×200 (:564). With 8px lane spacing it moves to (606,0)
with size 194×194 (:585). The retained tile survives direction changes.

Rust also runs the actual forward and backward 300px/5000px-per-second
touch flings through the Runtime with Flutter's 50 interpolated move
samples. The fourth keyed tile is materialized once, disappears
without rebuilding while away, then remounts with a new identity
and precisely one more builder call when it returns (:611).

The default clip optimization (:644) remains open. Flutter stores
HardEdge as the setting but emits no viewport clip when the grid fits.
The retained Incular clip is installed from the setting, not an
observed overflow decision. This batch verified the Flutter reference
only for that registration; the Rust display-list comparison is
separate from the direction and eviction evidence.

The grid lifecycle, grid layout, scroll keyboard and existing grid
compatibility binaries passed, 31 targeted Rust tests total.
