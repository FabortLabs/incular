# Upstream clip hit testing and hidden focus

Pinned Flutter 3.50.0 `ab2912035e`; Incular production `c647cb85`.

Four exact registrations in `widgets/clip_test.dart` passed on the
original Flutter file. The paired `flutter350_clip_hit_upstream_parity`
target reproduces four differences using runtime pointer down/up,
with a full-area tap child that gives the same hit extent:

- :155 Custom ClipPath: Flutter and Incular get the geometry once,
  and both deliver the center tap. Only Incular delivers the tap
  outside the clipping path.
- :178 and :200 ClipOval: center contact reaches both. A clipped
  corner reaches Incular with and without zero downstream opacity;
  it reaches neither Flutter variant.
- :225 ClipRect: exact callback checkpoints agree on when the
  clipper is consulted (initial, size change, delegate type change,
  ShouldReclip change). Only Incular delivers both outside taps,
  including the tap outside the updated custom rectangle.

These four failures are related manifestations of the raster-only
clip policy in Incular's neutral clips, not four independent renderer
bugs. This batch exercises routing rather than real GPU pixels.

Four original Visibility registrations passed (:406, :434, :461, :625).
The Rust `flutter_visibility_focus_parity` target adapts them using
normalized runtime key delivery and an external focus node rather
than the original Focus/Text adapter and native zero-size test view;
counted as partial. A separate Flutter extension checked key routing.

- Default replace unregisters the focus target and blocks hidden key
  delivery, but the detached Incular node reports focused both before
  and after a fresh request. Flutter reports false in both phases.
- With all five non-focus preservation flags enabled, Incular retains
  component identity as Flutter does, but Incular also retains host
  focus and delivers another key to the hidden child. Flutter does not.
- Visibility.maintain retains focus and delivery in both. A neutral
  zero-area placeholder completes both frames.

## Visibility focusability repair

Hidden Visibility content now loses focus and cannot receive keys unless
maintain_focusability is set. That option is independent of pointer
interactivity. Visibility.maintain enables it and Offstage keeps its
separate preserve-focus behavior. The original six-case runtime target
now covers the earlier reference pair and independent explicit opt-in
and Offstage controls. The pinned authored visibility_focusability probe
checks real keyboard delivery in all three configurations.

## Clip hit-testing repair

Hit testing now reuses the cached raster geometry. Custom rect, rounded
rect and path shapes constrain pointer routing; ovals use their ellipse
under zero opacity and even when paint clipping is off. A custom rect
preserves its shape when toggling Clip.none, resolves it once, and
excludes the right and bottom edges. Both pinned authored policy controls
and all six targeted Rust clip-hit tests passed, including the delegate
call counts. The clip/stack suites and both native Windows wgpu Stack
controls also passed.

## Default and childless ClipPath

Pinned originals :139 and :976 pass. The Rust port now mounts the
same childless ClipPath and updates the original render object from
default AntiAlias to HardEdge to None. It has no child and creates no
paint clip. When there is a child and no explicit path or clipper,
the default outline is the actual laid-out box. A second regression
completes the original zero-area centered Placeholder frame, then
checks the emitted bounds and mode of a nonempty scene through resize
and mode changes.

The earlier :155 custom-path hit test was also rerun against Flutter
and Incular without a redundant fallback path. Both report exactly
one geometry lookup, no outside tap, and one inside tap. This is a
recheck of the already registered mismatch, not another registration.
All 33 focused clip and shape tests pass. Path coverage now reaches
the display list; the general path renderer still uses a binary mask
and needs separate coverage and save-layer verification.

## Shape clip direction and actual layout size

The pinned :741 original and its Rust port match all six rebuild
checkpoints. The same shape resolves once without Directionality,
is reused on an equivalent rebuild, receives LTR and then RTL on
inherited updates, is reused on the identical RTL rebuild, and
resolves again with LTR. Both resolve a rectangle of 800 by 600
under the tight original viewport. A separate loose-constraints
probe observed a 100 by 40 child inside maximum bounds of 300 by
300. The old Incular outlined the entire maximum; the corrected
implementation and pinned Flutter outline the 100 by 40 child.

All 35 focused Rust tests passed. The cache recognizes a shared
shape instance; equality of distinct border values and generalized
path raster coverage were not covered by this recheck.

## Fractional rectangular clip modes

On a fractional 35.5 by 31.5 rectangular clip, pinned Flutter
distinguishes default HardEdge, None, ordinary AntiAlias and
AntiAliasWithSaveLayer. Windows wgpu previously treated both AA
modes as binary and showed zero partial pixels. The corrected
analytic box mask measures coverage in device pixels. The ordinary
mode permits red from an earlier draw at a partially covered edge;
the isolated mode retains partial coverage but does not leak red.
HardEdge and None retain their sampled interior/exterior controls.
The retained-mode test additionally detects an unintended relayout
or child repaint on coverage-only transitions.

All seven pinned authored raster checks, the focused native rounded,
elliptical and rectangular mode checks, seven native path controls,
and 31 retained/input tests pass. This is not an original golden
comparison. Hard-edge snapping at other fractional offsets, rotated
paths and destination-reading effect coverage remain separate.

## Childless rect clips and zero-sized fitted semantics

The pinned :66, :85, and :917 originals pass, with matching Rust
regressions. A ClipRect can now have no child and keeps the same
render object through its default HardEdge, AntiAlias, and None
updates without emitting any clip commands. A childless custom
ClipRect centered in a zero viewport remains size zero even though
its delegate describes a non-empty rectangle. A FittedBox with
zero-size image semantics survives layout, paint, and semantics
under a ClipRect and remains in the retained tree. All 39 focused
clip, input, and shape tests pass.

## Directional rounded clips

Both pinned originals (:826 and :862) and both Rust ports pass. All
four physical corners mirror for the original 15/30 start/end radii.
A tap at the origin changes the inherited direction on the same
render object. The first Rust implementation updated the render
property but still drew the old LTR corners: inherited render
invalidation did not update compositor-only properties. The fix
delivers that invalidation to the retained layer and the regression
now verifies the emitted RTL radii too. All 41 focused widget cases
pass. Elliptical corner radii and direction-required validation
without an ancestor are not covered by these two originals.

## Rotated Canvas hard rectangle

The pinned raw Canvas rect and equivalent path both leave a black
pixel at (12, 12), inside the bounding box but outside the rotated
rectangle. The old native Windows wgpu raw rect leaked white there,
while its path control was already correct. The repair uses the full
affine mask for a rotated or sheared hard rectangle and keeps the
simple scissor path for axis-aligned clips. Both renderer probes now
agree at all three samples, including the white center and the black
exterior. This authored probe does not add to the original count.

## Clip child attachment transitions

An authored Flutter frame sequence inserts, removes, and reinserts a
child for ClipRect, ClipRRect, and ClipPath without replacing the
render object. On the first native retained Rust port, the child was
present but a previously childless ClipRect never installed a clip
stage. Reconciliation now compares the required layer structure even
when the render properties have not changed. All three shapes emit a
clip only with a child and retain their original render IDs through
the full sequence. Both pinned authored and 52 focused Rust tests
pass; the original comparison count is unchanged.
