import 'dart:ui' as ui;

import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

const _gradient = LinearGradient(
  begin: AlignmentDirectional.centerStart,
  end: AlignmentDirectional.centerEnd,
  colors: [Color(0xffff0000), Color(0xff0000ff)],
);

Future<List<bool>> _sample(WidgetTester tester, GlobalKey boundaryKey) async {
  final boundary =
      boundaryKey.currentContext!.findRenderObject()! as RenderRepaintBoundary;
  return (await tester.runAsync(() async {
    final image = await boundary.toImage();
    try {
      final rgba = (await image.toByteData(
        format: ui.ImageByteFormat.rawRgba,
      ))!;
      return [
        for (final x in [10, image.width - 10])
          rgba.getUint8((20 * image.width + x) * 4) >
              rgba.getUint8((20 * image.width + x) * 4 + 2),
      ];
    } finally {
      image.dispose();
    }
  }))!;
}

void main() {
  testWidgets(
    'directional gradient shader refresh depends on the paint rectangle',
    (tester) async {
      final boundary = GlobalKey();
      Future<void> mount(
        TextDirection direction,
        double width, {
        bool fresh = false,
      }) => tester.pumpWidget(
        Directionality(
          textDirection: direction,
          child: Center(
            child: RepaintBoundary(
              key: boundary,
              child: SizedBox(
                width: width,
                height: 40,
                child: DecoratedBox(
                  key: ValueKey(fresh),
                  decoration: const BoxDecoration(gradient: _gradient),
                ),
              ),
            ),
          ),
        ),
      );

      await mount(TextDirection.ltr, 100);
      expect(await _sample(tester, boundary), [
        true,
        false,
      ], reason: 'fresh LTR');
      await mount(TextDirection.rtl, 100);
      expect(await _sample(tester, boundary), [
        true,
        false,
      ], reason: 'RTL at unchanged rectangle');
      await mount(TextDirection.rtl, 120);
      expect(await _sample(tester, boundary), [
        false,
        true,
      ], reason: 'RTL after size change');
      await mount(TextDirection.rtl, 100, fresh: true);
      expect(await _sample(tester, boundary), [
        false,
        true,
      ], reason: 'new RTL painter');
    },
  );
}
