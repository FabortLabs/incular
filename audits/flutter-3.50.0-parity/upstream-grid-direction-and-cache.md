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

## Viewport clip policy

The :644 case now pairs: the grid retains the HardEdge default, but
a single fitting row emits no clip. The focused Rust comparison had
emitted one extra clip before this change. The :668 and :735 controls
also verify default coverage on overflow and configured AntiAlias
coverage, including a retained replacement and the builder path.
Before the repair the replacement still painted a hard-edge clip.

Sliver viewport layout now carries visual overflow from the actual
sliver geometry to its existing retained clip stage. An overflowing
viewport emits the configured coverage; a fitting one bypasses the
stage without destroying it. Explicit None and AntiAliasWithSaveLayer
are also checked, as is shrinking an overflowing grid to fit.
The pinned authored overscroll case confirms that jumping a fitting
grid to −12 switches overflow on and restoring zero switches it off.

All three pinned originals and the authored probe passed. The seven
widget targets passed, with 46 total distinct Rust tests after the
two extra clip controls. These comparisons exercise the retained
display list; they do not assert a new WGPU pixel fixture.

## Padded and changing static grids

All three pinned `widgets/scrollable_grid_test.dart` originals
passed, as did three focused native runtime tests. A centered empty
one-lane grid mounts a position but has zero scroll extent. A 300px
grid with 100px top padding places its first tile at (250, 100)
with size 300×300. The onstage indices across the two actual touch
drags are [0, 1], [1, 2, 3], and [0, 1, 2]. Offscreen cached
elements are excluded from that comparison, as Flutter's test finder
excludes them. Changing the count from three to four updates the
retained grid and shows the fourth tile at (600, 0) at 200×200.
A drag over the fitting grid leaves its scroll offset at zero.
