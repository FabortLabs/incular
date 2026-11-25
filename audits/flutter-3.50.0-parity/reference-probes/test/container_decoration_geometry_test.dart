import 'dart:typed_data';
import 'dart:ui' as ui;

import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

const _red = Color(0xffff0000);
const _green = Color(0xff00ff00);
const _white = Color(0xffffffff);
const _frame = ValueKey('frame');

Future<ByteData> _render(
  WidgetTester tester,
  double width,
  double height,
  Widget child,
) async {
  await tester.pumpWidget(
    Directionality(
      textDirection: TextDirection.ltr,
      child: Align(
        alignment: Alignment.topLeft,
        child: RepaintBoundary(
          key: _frame,
          child: SizedBox(
            width: width,
            height: height,
            child: ColoredBox(color: _white, child: child),
          ),
        ),
      ),
    ),
  );
  final boundary = tester.renderObject<RenderRepaintBoundary>(
    find.byKey(_frame),
  );
  return (await tester.runAsync(() async {
    final image = await boundary.toImage();
    try {
      return (await image.toByteData(format: ui.ImageByteFormat.rawRgba))!;
    } finally {
      image.dispose();
    }
  }))!;
}

void main() {
  testWidgets('a plain BoxDecoration fills its Container without a child', (
    tester,
  ) async {
    final pixels = await _render(
      tester,
      40,
      40,
      Container(decoration: const BoxDecoration(color: _red)),
    );
    expect(pixels.getUint32((20 * 40 + 20) * 4), 0xff0000ff);
  });

  testWidgets(
    'Container clips a child to an inscribed circle only when opted in',
    (tester) async {
      for (final mode in [Clip.none, Clip.hardEdge]) {
        final pixels = await _render(
          tester,
          100,
          60,
          Container(
            clipBehavior: mode,
            decoration: const BoxDecoration(shape: BoxShape.circle),
            child: const ColoredBox(color: _green),
          ),
        );
        expect(pixels.getUint32((30 * 100 + 50) * 4), 0x00ff00ff);
        expect(
          pixels.getUint32((30 * 100 + 10) * 4),
          mode == Clip.none ? 0x00ff00ff : 0xffffffff,
        );
      }
    },
  );
  testWidgets('decoration can hit without clipping the child', (tester) async {
    for (final withChild in [false, true]) {
      for (final foreground in [false, true]) {
        for (final mode in [Clip.none, Clip.hardEdge]) {
          if (foreground && mode != Clip.none) {
            continue; // Container only clips to its background decoration.
          }
          const circle = BoxDecoration(shape: BoxShape.circle);
          var taps = 0;
          await tester.pumpWidget(
            Align(
              alignment: Alignment.topLeft,
              child: SizedBox(
                width: 100,
                height: 60,
                child: GestureDetector(
                  onTap: () => taps++,
                  child: Container(
                    decoration: foreground ? null : circle,
                    foregroundDecoration: foreground ? circle : null,
                    clipBehavior: mode,
                    child: withChild ? const ColoredBox(color: _green) : null,
                  ),
                ),
              ),
            ),
          );
          await tester.tapAt(const Offset(50, 30));
          expect(taps, 1);
          await tester.tapAt(const Offset(10, 30));
          expect(taps, withChild && mode == Clip.none ? 2 : 1);
        }
      }
    }
  });
  testWidgets('a childless Container fills the other bounded axis', (
    tester,
  ) async {
    const key = ValueKey('childless');
    await tester.pumpWidget(
      Align(
        alignment: Alignment.topLeft,
        child: SizedBox(
          width: 100,
          height: 60,
          child: Align(
            alignment: Alignment.topLeft,
            child: Container(key: key, width: 40),
          ),
        ),
      ),
    );
    expect(tester.getSize(find.byKey(key)), const Size(40, 60));
  });
  testWidgets(
    'Container pivot moves painted content without resizing the box',
    (tester) async {
      const container = ValueKey('transform-alignment container');
      const child = ValueKey('transform-alignment cyan');
      for (final (pivot, direction, x, y) in [
        (Alignment.centerRight, TextDirection.ltr, 150.0, 125.0),
        (AlignmentDirectional.centerEnd, TextDirection.rtl, 100.0, 125.0),
        (null, TextDirection.ltr, 100.0, 100.0),
      ]) {
        await tester.pumpWidget(
          Directionality(
            textDirection: direction,
            child: Stack(
              children: [
                Positioned(
                  left: 100,
                  top: 100,
                  child: Container(
                    key: container,
                    width: 100,
                    height: 100,
                    transform: Matrix4.diagonal3Values(0.5, 0.5, 1),
                    transformAlignment: pivot,
                    child: const ColoredBox(key: child, color: _green),
                  ),
                ),
              ],
            ),
          ),
        );
        expect(
          tester.getRect(find.byKey(container)),
          const Rect.fromLTWH(100, 100, 100, 100),
        );
        expect(tester.getRect(find.byKey(child)), Rect.fromLTWH(x, y, 50, 50));
      }
    },
  );
}
