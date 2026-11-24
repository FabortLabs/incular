import 'package:flutter/painting.dart';
import 'package:flutter_test/flutter_test.dart';

const _opaque = Color(0xff12589a);
const _transparent = Color(0x0012589a);
const _progress = [0.25, 0.5, 0.75];

void main() {
  test('BorderSide scale resets non-default stroke alignment', () {
    const cases = [
      (scale: 0.5, width: 2.0, style: BorderStyle.solid),
      (scale: 2.0, width: 8.0, style: BorderStyle.solid),
      (scale: 0.0, width: 0.0, style: BorderStyle.none),
      (scale: -1.0, width: 0.0, style: BorderStyle.none),
    ];
    for (final alignment in [
      BorderSide.strokeAlignCenter,
      BorderSide.strokeAlignOutside,
    ]) {
      final side = BorderSide(color: _opaque, width: 4, strokeAlign: alignment);
      for (final sample in cases) {
        final scaled = side.scale(sample.scale);
        final label = 'strokeAlign=$alignment, scale=${sample.scale}';
        expect(scaled.width, sample.width, reason: label);
        expect(scaled.style, sample.style, reason: label);
        expect(scaled.color, _opaque, reason: label);
        expect(scaled.strokeAlign, BorderSide.strokeAlignInside, reason: label);
        expect(scaled.strokeInset, sample.width, reason: label);
        expect(scaled.strokeOutset, 0.0, reason: label);
      }
    }
  });

  test('BorderSide lerp fades a None endpoint color in either direction', () {
    const none = BorderSide(color: _opaque, width: 2, style: BorderStyle.none);
    const solid = BorderSide(color: _opaque, width: 2);
    const transparentSolid = BorderSide(color: _transparent, width: 2);
    expect(BorderSide.lerp(none, solid, 0), none);
    expect(BorderSide.lerp(none, solid, 1), solid);
    expect(BorderSide.lerp(solid, none, 0), solid);
    expect(BorderSide.lerp(solid, none, 1), none);

    List<int> colors(BorderSide begin, BorderSide end) => [
      for (final t in _progress)
        BorderSide.lerp(begin, end, t).color.toARGB32(),
    ];
    const increasing = [0x4012589a, 0x8012589a, 0xbf12589a];
    const decreasing = [0xbf12589a, 0x8012589a, 0x4012589a];
    // With a transparent solid endpoint, ordinary color lerp is the control.
    expect(colors(transparentSolid, solid), increasing);
    expect(colors(solid, transparentSolid), decreasing);
    expect(colors(none, solid), increasing);
    expect(colors(solid, none), decreasing);
  });
}
