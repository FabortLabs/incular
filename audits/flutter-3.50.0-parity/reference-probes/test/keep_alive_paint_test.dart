import 'dart:ui' as ui;

import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

class Marker extends StatefulWidget {
  const Marker({super.key});

  @override
  State<Marker> createState() => MarkerState();
}

class MarkerState extends State<Marker> {
  @override
  Widget build(BuildContext context) =>
      const ColoredBox(color: Color(0xFFFF5722));
}

void main() {
  testWidgets(
    'unclipped parked kept-alive row stays mounted but never paints',
    (tester) async {
      final controller = ScrollController();
      addTearDown(controller.dispose);
      final marker = GlobalKey<MarkerState>();
      final boundary = GlobalKey();
      await tester.pumpWidget(
        Directionality(
          textDirection: TextDirection.ltr,
          child: RepaintBoundary(
            key: boundary,
            child: Stack(
              clipBehavior: Clip.none,
              children: [
                Positioned(
                  left: 0,
                  top: 400,
                  width: 100,
                  height: 180,
                  child: CustomScrollView(
                    controller: controller,
                    clipBehavior: Clip.none,
                    slivers: [
                      SliverList(
                        delegate: SliverChildBuilderDelegate(
                          (context, index) => KeepAlive(
                            keepAlive: index == 2,
                            child: SizedBox(
                              height: 50,
                              child: index == 2 ? Marker(key: marker) : null,
                            ),
                          ),
                          childCount: 40,
                          addAutomaticKeepAlives: false,
                          addRepaintBoundaries: false,
                          addSemanticIndexes: false,
                        ),
                      ),
                    ],
                  ),
                ),
              ],
            ),
          ),
        ),
      );
      final original = marker.currentState;
      expect(original, isNotNull);
      Future<List<int>> pixelAt(int y) async {
        final render =
            boundary.currentContext!.findRenderObject()!
                as RenderRepaintBoundary;
        return (await tester.runAsync(() async {
          final image = await render.toImage();
          try {
            final data = (await image.toByteData(
              format: ui.ImageByteFormat.rawRgba,
            ))!;
            final offset = 4 * (y * image.width + 10);
            return List.generate(
              4,
              (channel) => data.getUint8(offset + channel),
            );
          } finally {
            image.dispose();
          }
        }))!;
      }

      expect(await pixelAt(520), [255, 87, 34, 255]);
      controller.jumpTo(450);
      await tester.pump(const Duration(milliseconds: 16));
      expect(controller.offset, 450);
      expect(marker.currentState, same(original));
      expect((await pixelAt(65))[3], 0);

      controller.jumpTo(0);
      await tester.pump(const Duration(milliseconds: 16));
      expect(controller.offset, 0);
      expect(marker.currentState, same(original));
      expect(await pixelAt(520), [255, 87, 34, 255]);
    },
  );
}
