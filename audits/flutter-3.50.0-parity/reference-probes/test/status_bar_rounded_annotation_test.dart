import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('status-bar metadata ignores a clipped foreground corner', (
    tester,
  ) async {
    const blue = Color(0xff2196f3);
    const green = Color(0xff4caf50);
    const padding = FakeViewPadding(top: 50, bottom: 108);
    tester.view
      ..viewPadding = padding
      ..padding = padding
      ..devicePixelRatio = 2
      ..physicalSize = const Size(960, 1920);
    addTearDown(tester.view.reset);
    addTearDown(() async {
      SystemChrome.setSystemUIOverlayStyle(const SystemUiOverlayStyle());
      await pumpEventQueue();
    });

    Widget region(Color color) => AnnotatedRegion<SystemUiOverlayStyle>(
      value: SystemUiOverlayStyle(statusBarColor: color),
      child: ColoredBox(color: color, child: const SizedBox.expand()),
    );

    for (final (inset, expected) in [(1.0, blue), (6.0, green)]) {
      await tester.pumpWidget(
        Directionality(
          textDirection: TextDirection.ltr,
          child: Stack(
            children: [
              region(blue),
              Positioned(
                left: 240 - inset,
                top: 12.5 - inset,
                width: 20,
                height: 20,
                child: ClipRRect(
                  borderRadius: BorderRadius.circular(8),
                  child: region(green),
                ),
              ),
            ],
          ),
        ),
      );
      await tester.pumpAndSettle();
      expect(SystemChrome.latestStyle?.statusBarColor, expected);
    }
  }, variant: TargetPlatformVariant.mobile());
}
