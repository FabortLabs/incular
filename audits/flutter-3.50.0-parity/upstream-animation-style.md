# AnimationStyle interpolation (backfill A01)

All eight originals in animation/animation_style_test.dart passed against pinned Flutter 3.50.0. The focused Incular binary has three passing controls and three intentional assertion failures.

| Registration | Evidence |
| --- | --- |
| :12 | Default value and equality align under an empty override. Dart hashCode is not compared. |
| :17 | Selecting overrides for all four fields yields the specified forward/reverse durations and curves. |
| :36 | Partial override keeps the original reverse curve/duration while replacing the forward pair. |
| :51 | Null/null, equal style and interpolation endpoints preserve their values. Rust value semantics do not reproduce Dart identical. |
| :57 | Between 1s and 2s Flutter blends both durations to [1166667,1333333,1500000,1666667,1833333] microseconds. Incular steps from 1000000 to 2000000 at the midpoint. Curve blends also take the old or new curve instead of blending their evaluated outputs; the focused test uses Incular's own endpoint samples to isolate interpolation from solver precision. |
| :104 | A missing duration should be treated as zero inside the interpolation. At 0/0.25/0.5/1, Flutter's behavior is [2s,1.5s,1s,null], while Incular gives [2s,2s,null,null]. |
| :113, :126 | Reference only. Rust's Debug surface is not Dart's DiagnosticPropertiesBuilder. |

Run: cargo test --locked -p incular-animation --test flutter350_animation_style_parity -- --nocapture.
