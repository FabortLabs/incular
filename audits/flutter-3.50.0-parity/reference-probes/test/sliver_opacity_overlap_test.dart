import 'dart:ui' as ui;

import 'package:flutter/animation.dart' show AlwaysStoppedAnimation;
import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

Widget rows() => SliverList.list(
  children: [
    Transform.translate(
      offset: const Offset(0, 20),
      child: const ColoredBox(
        color: Color(0xFFFFFFFF),
        child: SizedBox(height: 40),
      ),
    ),
    const ColoredBox(color: Color(0xFF000000), child: SizedBox(height: 40)),
  ],
);

Future<void> checkGroupedPixels(
  WidgetTester tester,
  Widget sliver,
  Finder finder,
) async {
  final key = GlobalKey();
  await tester.pumpWidget(
    Directionality(
      textDirection: TextDirection.ltr,
      child: Align(
        alignment: Alignment.topLeft,
        child: RepaintBoundary(
          key: key,
          child: SizedBox(
            width: 100,
            height: 100,
            child: CustomScrollView(slivers: [sliver]),
          ),
        ),
      ),
    ),
  );
  final render = tester.renderObject<RenderSliver>(finder);
  expect((render.debugLayer as OpacityLayer).alpha, 128);
  final boundary =
      key.currentContext!.findRenderObject()! as RenderRepaintBoundary;
  final output = await tester.runAsync(() async {
    final image = await boundary.toImage();
    try {
      final data = (await image.toByteData(
        format: ui.ImageByteFormat.rawStraightRgba,
      ))!;
      int alpha(int x, int y) => data.getUint8((y * image.width + x) * 4 + 3);
      return [alpha(20, 30), alpha(20, 50), alpha(20, 70), alpha(20, 90)];
    } finally {
      image.dispose();
    }
  });
  expect(output, [128, 128, 128, 0]);
}

void main() {
  testWidgets('overlapping sliver boxes are attenuated once as a group', (
    tester,
  ) async {
    await checkGroupedPixels(
      tester,
      SliverOpacity(opacity: 0.5, sliver: rows()),
      find.byType(SliverOpacity),
    );
  });

  testWidgets('both animated sliver variants attenuate overlapping rows once', (
    tester,
  ) async {
    await checkGroupedPixels(
      tester,
      SliverFadeTransition(
        opacity: const AlwaysStoppedAnimation<double>(0.5),
        sliver: rows(),
      ),
      find.byType(SliverFadeTransition),
    );
    await checkGroupedPixels(
      tester,
      SliverAnimatedOpacity(
        opacity: 0.5,
        duration: const Duration(seconds: 1),
        sliver: rows(),
      ),
      find.byType(SliverAnimatedOpacity),
    );
  });
}
