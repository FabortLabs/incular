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
