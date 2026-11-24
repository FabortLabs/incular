# SingleChildScrollView: drag, controller attachment and overflow

Pinned Flutter passed five selected original registrations. The Incular
runtime target passes four, with one paint-clip assertion red:

- :52 partial mismatch: both axes produce the intended clip on a 0.1 px
  layout overflow, but Incular also clips when content exactly matches
  the viewport. The retained display list reports four clips instead of
  the expected false/true/false/true. Flutter's private painting context
  and descriptor inspection are not substituted by the Rust fixture.
- :146 fully paired: the node starts at (0,0) and moves to (0,-200)
  after the same diagonal touch, including the testing helper's 20 px
  slop waypoint. This exercises actual runtime gesture routing.
- :159 partial match: switching controllers updates viewport ownership
  and offset, and writes to the previous controller do not move it.
  Incular does not have the custom subclass of ScrollPosition asserted
  in the original.
- :195 fully paired: changing from a controllerless child to an explicit
  controller inside a dirty LayoutBuilder while width changes from 750
  to 700 completes layout without an assertion. An extra Rust check
  confirms the controller attaches to the 1400-pixel scroll range.
- :1087 partial match: the visible viewport reaches Size.zero without a
  layout exception at zero constraints. No native monitor is resized.

Reference: five exact names in `widgets/single_child_scroll_view_test.dart`.
Rust: `cargo test --locked -j 2 -p incular-runtime --test flutter350_single_child_scroll_basics -- --test-threads=1`.

Remediation: the single-child viewport now attaches its rect clip only
when the translated layout bounds of that child exceed the viewport.
The exact-fit and 0.1 px overflow cases now agree in both dimensions;
all five focused runtime tests pass. This does not claim Flutter's
private paint-context or antialias inspection.

The pinned Flutter and Windows wgpu pixel probes agree over six retained
frames. An in-bounds child's paint-only shift stays visible past the
viewport; identical pixels produced by an overflowing layout child
are clipped. Clip.none keeps the same content visible and re-enabling
HardEdge clips again. The four named Windows wgpu targets also cover
existing Stack, IndexedStack and invisible-border controls.
