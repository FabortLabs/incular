import 'dart:ui' as ui;

import 'package:flutter/rendering.dart' show RenderRepaintBoundary;
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

const _blue = Color(0xff0000ff);
const _black = Color(0xff000000);

void main() {
  testWidgets('a translated lens samples its focal point and clips to its own box', (
    tester,
  ) async {
    final boundaryKey = GlobalKey();
    Future<List<List<int>>> capture(bool lens) async {
      await tester.pumpWidget(
        Directionality(
          textDirection: TextDirection.ltr,
          child: Center(
            child: RepaintBoundary(
              key: boundaryKey,
              child: SizedBox.square(
                dimension: 8,
                child: Stack(
                  children: <Widget>[
                    const Positioned.fill(child: ColoredBox(color: _blue)),
                    const Positioned(
                      left: 4,
                      top: 4,
                      width: 4,
                      height: 4,
                      child: ColoredBox(color: _black),
                    ),
                    if (lens)
                      const Positioned(
                        left: 2,
                        top: 2,
                        child: RawMagnifier(
                          size: Size(4, 4),
                          focalPointOffset: Offset(1, 1),
                          magnificationScale: 2,
                          clipBehavior: Clip.hardEdge,
                        ),
                      ),
                  ],
                ),
              ),
            ),
          ),
        ),
      );
      final boundary = tester.renderObject<RenderRepaintBoundary>(find.byKey(boundaryKey));
      return (await tester.runAsync(() async {
        final image = await boundary.toImage();
        try {
          final data = (await image.toByteData(format: ui.ImageByteFormat.rawRgba))!;
          return <List<int>>[
            for (final (x, y) in [(3, 3), (3, 4), (1, 1), (7, 7)])
              List<int>.generate(4, (channel) => data.getUint8((y * image.width + x) * 4 + channel)),
          ];
        } finally {
          image.dispose();
        }
      }))!;
    }

    const blue = [0, 0, 255, 255];
    const black = [0, 0, 0, 255];
    expect(await capture(false), [blue, blue, blue, black]);
    expect(await capture(true), [black, black, blue, black]);
  });
}
