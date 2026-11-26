# TextStyle tweening across unspecified fields

Flutter 3.50.0's pinned `painting/text_style_test.dart` originals at
:711, :717 and :724 passed. Incular now rejects a debug interpolation
between mixed `inherit` values if an interpolated field is missing
on both sides. The diagnostic identifies the missing fields, and a
style providing every needed value on either side animates normally.
This pairs the original :717 and :724 behavior. The :711 value
behavior passes, but remains partial because the Rust trait returns
a style value, not a nullable Dart object with reference identity.

For two present style descriptors, a font size, word or letter
spacing, line height or decoration thickness specified only on one
side now stays fixed throughout the interpolation. Earlier, sizes
and spacing blended against synthesized defaults and a line height
appeared abruptly at halfway. Two specified numbers support
extrapolation. Font family and font style can remain unspecified
on the selected side, so they continue to inherit there. An isolated
plain text or decoration color fades in alpha without blending its
RGB channels toward the default color. Solid foreground/background
paint precedence remains unchanged.

The Flutter probe checks missing values and both directions,
midpoint inheritance, alpha and extrapolation. It also drives
a mounted `AnimatedDefaultTextStyle` from empty to size 26 with an
explicit color. The Rust widget probe inspects the corresponding
painted glyph run: 26 logical pixels and the expected alpha at
250, 500, 750 and 1000 ms. Five authored Flutter and 59 focused
native checks passed. Incular's absolute-pixel line-height extension
remains supported; switching between a pixel and multiplier height
retains its discrete transition. Dart's full diagnostic tree and
nullable whole-style interpolation are not claimed.

```sh
cargo test --locked -j 1 -p incular-text --test text_style_nullable_tween --test style -p incular-widgets --test flutter350_text_style_tween --test flutter350_text_foreground --test flutter350_text_background --test implicit_animations --test explicit_transitions -- --test-threads=1
bash audits/flutter-3.50.0-parity/run-reference-linux.sh probe text_style_nullable_tween_test.dart
```
