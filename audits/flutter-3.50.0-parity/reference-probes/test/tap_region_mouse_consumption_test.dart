import 'package:flutter/gestures.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  for (final button in [kSecondaryButton, kTertiaryButton]) {
    testWidgets(
      'outside region claims a matched button only when enabled: $button',
      (tester) async {
        const target = ValueKey<String>('outside gesture');
        const child = SizedBox(width: 70, height: 32);
        var notifications = 0;
        var gestures = 0;
        var primary = 0;

        Widget scene(bool consume) => Directionality(
          textDirection: TextDirection.ltr,
          child: TapRegionSurface(
            child: Row(
              children: [
                GestureDetector(
                  key: target,
                  behavior: HitTestBehavior.opaque,
                  onTap: () => primary++,
                  onSecondaryTap: () => gestures++,
                  onTertiaryTapUp: (_) => gestures++,
                  child: child,
                ),
                TapRegion(
                  consumeOutsideTaps: consume,
                  onTapOutside: (event) {
                    expect(event.buttons, button);
                    notifications++;
                  },
                  child: child,
                ),
              ],
            ),
          ),
        );

        for (final consume in [false, true, false]) {
          notifications = 0;
          gestures = 0;
          await tester.pumpWidget(scene(consume));
          final gesture = await tester.startGesture(
            tester.getCenter(find.byKey(target)),
            pointer: 42,
            kind: PointerDeviceKind.mouse,
            buttons: button,
          );
          await gesture.up();
          await gesture.removePointer();
          expect(notifications, 1);
          expect(gestures, consume ? 0 : 1);
          expect(primary, 0);
        }
      },
    );
  }
}
