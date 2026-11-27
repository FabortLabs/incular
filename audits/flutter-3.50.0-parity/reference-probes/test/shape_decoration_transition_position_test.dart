import 'dart:ui' as ui;

import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

const green = Color(0xFF00FF00);
const red = Color(0xFFFF0000);
const shape = ShapeDecoration(shape: CircleBorder(), color: green);
const box = BoxDecoration(color: green);

Future<List<int>> centerPixel(WidgetTester tester, GlobalKey boundary) async {
  final render =
      boundary.currentContext!.findRenderObject()! as RenderRepaintBoundary;
  return (await tester.runAsync(() async {
    final image = await render.toImage();
    try {
      final pixels = (await image.toByteData(
        format: ui.ImageByteFormat.rawRgba,
      ))!;
      final at = 4 * (20 * image.width + 20);
      return List<int>.generate(4, (i) => pixels.getUint8(at + i));
    } finally {
      image.dispose();
    }
  }))!;
}

Widget scene(
  GlobalKey boundary,
  Animation<Decoration> decoration,
  DecorationPosition position,
  Widget child,
) {
  return Directionality(
    textDirection: TextDirection.ltr,
    child: Center(
      child: RepaintBoundary(
        key: boundary,
        child: DecoratedBoxTransition(
          decoration: decoration,
          position: position,
          child: child,
        ),
      ),
    ),
  );
}

void main() {
  testWidgets(
    'a shape transition honors foreground and background without taking child taps',
    (tester) async {
      final boundary = GlobalKey();
      var taps = 0;
      final child = GestureDetector(
        behavior: HitTestBehavior.opaque,
        onTap: () => taps++,
        child: const SizedBox(
          width: 40,
          height: 40,
          child: ColoredBox(color: red),
        ),
      );
      const animation = AlwaysStoppedAnimation<Decoration>(shape);
      await tester.pumpWidget(
        scene(boundary, animation, DecorationPosition.foreground, child),
      );
      expect(await centerPixel(tester, boundary), [0, 255, 0, 255]);
      await tester.tapAt(const Offset(400, 300));
      expect(taps, 1);

      await tester.pumpWidget(
        scene(boundary, animation, DecorationPosition.background, child),
      );
      expect(await centerPixel(tester, boundary), [255, 0, 0, 255]);
      await tester.tapAt(const Offset(400, 300));
      expect(taps, 2);
    },
  );

  testWidgets(
    'a box-to-shape transition stays foreground at both ends and mid-tween',
    (tester) async {
      final boundary = GlobalKey();
      final animation = AnimationController(vsync: tester);
      addTearDown(animation.dispose);
      final tween = DecorationTween(begin: box, end: shape).animate(animation);
      await tester.pumpWidget(
        scene(
          boundary,
          tween,
          DecorationPosition.foreground,
          const SizedBox(width: 40, height: 40, child: ColoredBox(color: red)),
        ),
      );
      for (final value in <double>[0, 0.25, 0.5, 0.75, 1]) {
        animation.value = value;
        await tester.pump();
        expect(await centerPixel(tester, boundary), [
          0,
          255,
          0,
          255,
        ], reason: '$value');
      }
    },
  );
}
