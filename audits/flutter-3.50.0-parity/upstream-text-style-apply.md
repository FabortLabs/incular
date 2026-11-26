# Applying text styles and clearing inherited paint

Pinned original `painting/text_style_test.dart:618` and its native
comparison passed. `TextStyle::apply_options` is the typed equivalent
for the tested optional fields. It replaces typed shadow, feature and
variation lists, locale, baseline, leading distribution, font style and
decoration choices without dropping unspecified source settings. The
existing positional helper delegates to it and keeps paint precedence.

Numeric adjustments multiply before applying the delta, including when
the intermediate number is below zero. Missing metrics retain Flutter's
debug guard; a missing height is exempt and stays absent. A variable
weight is mapped to its legacy hundred-index before applying a requested
shift, clamped from 100 to 900. The tests cover signed spacing and both
index extremes.

Flutter's explicit zero text height is the natural-font sentinel, not
a zero-pixel line. It overrides an inherited height and does not change
under apply. The Flutter-style height builder now preserves that
meaning and treats a nonzero height like 5 as a multiplier. The actual
baseline advance is 100 pixels at 20px font size with multiplier 5 in
both tests, while the natural override returns to each font's original
metrics. The separate absolute-pixel Incular API is unchanged.

An explicitly empty shadow list now clears a shadow inherited from
the surrounding text style. The native mounted test verifies that the
shadow glyphs disappear while the foreground glyph positions stay put.
The pinned probe verifies default, explicit-empty and tweened-empty
shadow lists. Four authored Flutter and 30 focused native tests passed.

The Dart named `package` option, including requalifying an existing
package independently from a family or fallback name, is separate.
Incular callers can already pass a canonical packaged family.

```sh
cargo test --locked -j 1 -p incular-text --test text_style_apply --test flutter350_text_style_contracts --test text_style_nullable_tween -p incular-widgets --test text_shadow_alignment --test flutter350_text_foreground --test flutter350_text_background -- --test-threads=1
bash audits/flutter-3.50.0-parity/run-reference-linux.sh probe text_style_apply_metrics_test.dart
```
