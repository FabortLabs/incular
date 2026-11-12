# Flutter parity harnesses

Two harnesses compare Incular with Flutter 3.41.7 (the SDK the reference
values were taken from). Both are opt-in: a plain `cargo test` skips them.

| Harness | Compares | Flutter side | Incular side | Run |
| --- | --- | --- | --- | --- |
| Frames | Geometry, transform, opacity, fill and sampled values of probed widgets, frame by frame on a fake clock | `flutter/test/frames/` | `incular/tests/frames/` | `harness/run-frames.sh [name]` |
| Pixels | Rendered pixels of static scenes at 1, 1.25, 1.5 and 2x | `flutter/lib/main.dart` (Windows app) | `incular/tests/pixels.rs` (WGPU) | `harness/run-pixels.sh` |

```
harness/
  flutter/                 Flutter reference project (package parity_ref)
    fonts/Inter-Regular.ttf  the one font both sides load
    lib/main.dart            pixel scenes (Windows app, writes PNGs)
    test/frames/             frame scenarios + recorder.dart
    test/values/             one-off scalar reference values (curves, ticker futures, transitions)
  incular/                 Rust package incular-parity-harness
    src/lib.rs               frame recorder, comparer and report
    tests/frames/            frame scenarios (main.rs registers the modules)
    tests/pixels.rs          pixel scenes
  out/                     generated, git-ignored (override with INCULAR_PARITY_DIR)
    frames/{flutter,incular}/<scenario>.json, frames/report.md
    pixels/{flutter,incular,diff}/<scene>@<dpr>.png
```

`flutter` must be on `PATH`, or set `FLUTTER` to the executable (here
`C:/Manual Installs/flutter/bin/flutter`).

## Frame-by-frame harness

A scenario is written twice, once per framework, under the same name. Both
twins mount a scene, advance a fake clock in 16 ms frames (or explicit
pumps), drive input stamped with that clock, and after every frame record
each probe:

| Field | Flutter | Incular |
| --- | --- | --- |
| `rect` | `MatrixUtils.transformRect(getTransformTo(null), paintBounds)` | `element_geometry` transform applied to the size |
| `transform` | `[a, b, c, d, tx, ty]` from `getTransformTo(null)` | `Transform::coefficients()` of the world transform |
| `size` | `paintBounds` size | render size |
| `opacity` | product of `RenderOpacity`, `RenderAnimatedOpacity`, sliver opacities, `RenderOffstage` above the probe | product of `RenderKind::Opacity` (controller value when driven) and hidden `Visibility` above the probe |
| `color` | first `ColoredBox` color or `DecoratedBox` `BoxDecoration.color` at or below the probe | first `RenderKind::Box` color or solid `Decorated` background at or below the probe |
| values | `h.value(name, () => ...)` | `h.value(name, move \|\| ...)` |

The comparer reports, for each scenario, structural problems (frame count
or timestamps differ) and, per field, the first diverging frame, both
values there, how many frames diverge and the largest difference.
Tolerances: 0.01 px, 1e-4 for transform scale/rotation, 0.002 opacity,
1 color step, 1e-3 for values; override with `PARITY_TOL_PX`,
`PARITY_TOL_TRANSFORM`, `PARITY_TOL_OPACITY`, `PARITY_TOL_COLOR`,
`PARITY_TOL_VALUE`.

### Running

```sh
harness/run-frames.sh                 # every scenario
harness/run-frames.sh list_fling      # names containing a substring
```

Or by hand: `flutter test test/frames` in `harness/flutter` writes the
Flutter traces; `INCULAR_PARITY=1 cargo test -p incular-parity-harness --test frames`
writes the Incular traces, compares, and writes `harness/out/frames/report.md`.
`PARITY_SCENARIO=<substring>` runs only matching scenarios and writes
`report-<substring>.md` instead, so concurrent filtered runs do not
overwrite each other; `PARITY_STRICT=1` fails on a divergence or a missing
Flutter trace.

### Writing a scenario

1. Add the Flutter twin to a `harness/flutter/test/frames/<area>_test.dart`
   file (a new file needs `import 'recorder.dart';` and a `main()`):

   ```dart
   frameScenario('animated_align_basic', (h) async {
     await h.mount((phase) => AnimatedAlign(
           alignment: phase == 0 ? Alignment.topLeft : Alignment.bottomRight,
           duration: const Duration(milliseconds: 300),
           child: h.probe('box', const SizedBox(width: 50, height: 50, child: ColoredBox(color: Color(0xFF2196F3)))),
         ));
     await h.frames(2);
     h.setPhase(1);
     await h.frames(25);
   });
   ```

2. Add the Incular twin to `harness/incular/tests/frames/<area>.rs` and
   return it from that module's `scenarios()` (a new module is declared and
   collected in `tests/frames/main.rs`):

   ```rust
   fn animated_align_basic(h: &mut FrameHarness) {
       h.mount(|phase| {
           AnimatedAlign::new(
               if phase == 0 { Alignment::TOP_LEFT } else { Alignment::BOTTOM_RIGHT },
               Duration::from_millis(300),
               probe("box", SizedBox::new().width(50.).height(50.).child(colored(BLUE))),
           )
           .into()
       });
       h.frames(2);
       h.set_phase(1);
       h.frames(25);
   }
   ```

3. Run `harness/run-frames.sh animated_align_basic`.

Rules that keep the twins comparable:

- Same widget tree, same constants, same timeline, step for step. Use the
  Flutter widget's defaults on both sides only when they are documented to
  be equal; otherwise pass values explicitly.
- The scene fills an 800x600 logical surface at device pixel ratio 1, under
  `Directionality(ltr)` and a root text style of Inter 14 black. Text must
  use Inter (the default family on both sides is set to it by the root
  style); never rely on another font.
- `h.probe(name, child)` keys `child` with `probe:<name>`, as Flutter's
  `KeyedSubtree` and Incular's `KeyedSubtree` both do. Keys take part in
  reconciliation, so probe inside a widget whose own key matters (an
  `AnimatedSwitcher` child, a reordered list item), not around it.
- Probe the subtree whose render object you mean: in Flutter the probe
  records `KeyedSubtree`'s first descendant render object, in Incular the
  keyed widget's own render object. A wrapper that is a render object on
  one side and not the other (for example `Container` with only `color`,
  which is a `ColoredBox` in Flutter) records the same box either way, but
  check the frame-0 sizes match before trusting a divergence.
- Phase changes rebuild the scene in the next frame. Input is delivered
  immediately, stamped with the current fake time; a frame must be pumped
  for its effect to show. Touch is Flutter's default test pointer kind;
  `hover`/`wheel` use a mouse.
- Flutter `ColoredBox` without a child is `ColoredBox::new(color, SizedBox::shrink())`.
- Anything a probe cannot show (a controller value, a scroll offset, a
  status) is recorded with `h.value`.

When a scenario diverges, first rule out a twin mismatch (frame 0 must
match, or differ for a reason you can name), then read the Flutter source
for the behavior (`C:/Manual Installs/flutter/packages/flutter/lib/src`).

### Limits

- The recorder sees geometry, opacity, the first solid fill and sampled
  values. It does not see clips, shadows, gradients, text glyphs, custom
  painter output or paint order; check those with `h.value` (for example a
  painter's progress) or with the pixel harness.
- `RenderKind` coverage for opacity and fill on the Incular side is in
  `harness/incular/src/lib.rs` (`opacity`, `fill`); extend both sides
  together when a widget paints through another kind.
- Keyboard input: Flutter has `h.key(LogicalKeyboardKey)`; on the Incular
  side send a `KeyboardEvent` with `h.send(InputEvent::Key(...))`.

## Pixel harness

`harness/run-pixels.sh` runs the Flutter Windows app, which renders every
scene in `lib/main.dart` into a 160x160 `RepaintBoundary` at each device
pixel ratio and writes `out/pixels/flutter/<scene>@<dpr>.png`, then renders
the same scenes through Incular's WGPU renderer and prints per-scene
statistics (max, mean, share of pixels off by more than 2 and 16), writing
amplified diffs to `out/pixels/diff`. `--incular-only` reuses existing
Flutter captures. Scenes must match one for one between `lib/main.dart` and
`incular/tests/pixels.rs`. Diagnostics:
`INCULAR_PARITY_PROBE="blend@1.0.png:20,20;21,20"` prints both pixels,
`INCULAR_PARITY_BANDS="text@1.0.png:8-28"` prints row-band sums,
`INCULAR_PARITY_TONE="text@1.0.png"` prints the tone mapping.

## Scalar reference values

`flutter/test/values/*_test.dart` print reference numbers (curve samples,
spline samples, ticker future outcomes, transition values) that the
Incular parity tests assert (for example
`crates/incular-animation/tests/flutter_animation_parity.rs`). Run
one with `flutter test test/values/<file>` and copy the printed values.
