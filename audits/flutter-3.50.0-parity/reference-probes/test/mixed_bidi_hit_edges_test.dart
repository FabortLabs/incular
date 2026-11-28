import 'package:flutter/painting.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('hit testing chooses the visual glyph on each side of a bidi boundary', () {
    for (final (source, direction) in [
      ('ABCאבגDEF', TextDirection.ltr),
      ('אבגABCדהו', TextDirection.rtl),
    ]) {
      final painter = TextPainter(
        text: TextSpan(
          text: source,
          style: const TextStyle(fontFamily: 'FlutterTest', fontSize: 10),
        ),
        textDirection: direction,
      )..layout();
      expect(painter.width, 90);
      const probes = [
        -100.0,
        0.0,
        4.0,
        5.0,
        8.0,
        10.0,
        12.0,
        28.0,
        29.9,
        30.0,
        30.1,
        32.0,
        35.0,
        38.0,
        42.0,
        58.0,
        59.9,
        60.0,
        60.1,
        62.0,
        65.0,
        88.0,
        90.0,
        100.0,
      ];
      final hits = [
        for (final x in probes) painter.getPositionForOffset(Offset(x, 5)),
      ];
      print(
        '${direction.name}: ${hits.map((p) => "${p.offset}${p.affinity == TextAffinity.upstream ? 'u' : 'd'}").join(' ')}',
      );
      final actual = hits
          .map(
            (p) =>
                "${p.offset}${p.affinity == TextAffinity.upstream ? 'u' : 'd'}",
          )
          .join(' ');
      expect(
        actual,
        direction == TextDirection.ltr
            ? '0d 0d 0d 1u 1u 1d 1d 3u 3u 6u 6u 6u 5d 5d 5u 3d 3d 6d 6d 6d 7u 9u 9u 9u'
            : '9u 9u 9u 8d 8d 8u 8u 6d 6d 3d 3d 3d 4u 4u 4d 6u 6u 3u 3u 3u 2d 0d 0d 0d',
      );
      painter.dispose();
    }
  });
}
