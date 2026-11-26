import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets(
    'a superellipse paints a directional clip without narrowing box hits',
    (tester) async {
      var taps = 0;
      final target = GlobalKey();
      Widget build(TextDirection direction, Clip clip) => Directionality(
        textDirection: direction,
        child: Center(
          child: ClipRSuperellipse(
            key: target,
            borderRadius: const BorderRadiusDirectional.only(
              topStart: Radius.elliptical(20, 14),
            ),
            clipBehavior: clip,
            child: RepaintBoundary(
              child: GestureDetector(
                behavior: HitTestBehavior.opaque,
                onTap: () => taps++,
                child: const ColoredBox(
                  color: Color(0xffff0000),
                  child: SizedBox(width: 60, height: 30),
                ),
              ),
            ),
          ),
        ),
      );

      const expected = Size(60, 30);
      for (final direction in <TextDirection>[
        TextDirection.ltr,
        TextDirection.rtl,
      ]) {
        await tester.pumpWidget(build(direction, Clip.antiAlias));
        expect(tester.getSize(find.byKey(target)), expected);
        final clip = tester.layers
            .whereType<ClipRSuperellipseLayer>()
            .single
            .clipRSuperellipse!;
        expect(clip.outerRect.size, expected);
        final left = clip.outerRect.topLeft + const Offset(1, 1);
        final right = clip.outerRect.topRight + const Offset(-1, 1);
        expect(clip.contains(left), direction == TextDirection.rtl);
        expect(clip.contains(right), direction == TextDirection.ltr);

        final rect = tester.getRect(find.byKey(target));
        final before = taps;
        await tester.tapAt(rect.topLeft + const Offset(1, 1));
        await tester.tapAt(rect.topRight + const Offset(-1, 1));
        expect(taps, before + 2);
      }
      await tester.pumpWidget(build(TextDirection.rtl, Clip.none));
      expect(tester.layers.whereType<ClipRSuperellipseLayer>(), isEmpty);
      expect(tester.getSize(find.byKey(target)), expected);
    },
  );
}
