import 'dart:ui' as ui;

import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

const Color blue = Color(0xff0000ff);
const Color green = Color(0xff00ff00);

PageRoute<void> route(Color color) => PageRouteBuilder<void>(
  transitionDuration: const Duration(milliseconds: 100),
  reverseTransitionDuration: const Duration(milliseconds: 100),
  pageBuilder: (_, _, _) => ColoredBox(color: color),
  transitionsBuilder: (_, animation, _, child) =>
      FadeTransition(opacity: animation, child: child),
);

void main() {
  testWidgets('an entering route paints nothing at zero opacity', (tester) async {
    final key = GlobalKey<NavigatorState>();
    final boundaryKey = GlobalKey();
    await tester.pumpWidget(
      Directionality(
        textDirection: TextDirection.ltr,
        child: Center(
          child: SizedBox(
            width: 80,
            height: 80,
            child: RepaintBoundary(
              key: boundaryKey,
              child: Navigator(key: key, onGenerateRoute: (_) => route(blue)),
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    Future<List<int>> pixel() async {
      final boundary = tester.renderObject<RenderRepaintBoundary>(
        find.byKey(boundaryKey),
      );
      return (await tester.runAsync(() async {
        final image = await boundary.toImage(pixelRatio: 1.0);
        try {
          final data = (await image.toByteData(
            format: ui.ImageByteFormat.rawRgba,
          ))!;
          return List.generate(4, data.getUint8);
        } finally {
          image.dispose();
        }
      }))!;
    }

    expect(await pixel(), [0, 0, 255, 255]);
    key.currentState!.push(route(green));
    await tester.pump();
    expect(find.byType(FadeTransition), findsWidgets);
    expect(await pixel(), [0, 0, 255, 255]);

    await tester.pump(const Duration(milliseconds: 50));
    final halfway = await pixel();
    expect(halfway[1], greaterThan(0));
    expect(halfway[2], greaterThan(0));
    expect(halfway[3], 255);

    await tester.pumpAndSettle();
    expect(await pixel(), [0, 255, 0, 255]);
  });
}
