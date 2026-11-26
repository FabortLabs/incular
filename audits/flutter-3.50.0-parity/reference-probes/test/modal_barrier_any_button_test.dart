import 'package:flutter/gestures.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('a modal barrier dismisses after any stable contact releases', (
    WidgetTester tester,
  ) async {
    var dismissals = 0;
    await tester.pumpWidget(
      Directionality(
        textDirection: TextDirection.ltr,
        child: ModalBarrier(
          color: const Color(0xFF000000),
          onDismiss: () => dismissals++,
        ),
      ),
    );
    var contact = 0;
    for (final mask in [
      kPrimaryButton,
      kSecondaryButton,
      kTertiaryButton,
      kBackMouseButton,
      kForwardMouseButton,
      kSecondaryButton | kTertiaryButton,
    ]) {
      final press = await tester.startGesture(
        const Offset(400, 300),
        buttons: mask,
      );
      await tester.pump();
      expect(dismissals, contact, reason: 'press with mask $mask');
      await press.up();
      expect(dismissals, ++contact, reason: 'release with mask $mask');
    }
  });
  testWidgets(
    'a translucent drag competes with a barrier tap and cancel never dismisses',
    (WidgetTester tester) async {
      var dismissals = 0;
      var drags = 0;
      await tester.pumpWidget(
        Directionality(
          textDirection: TextDirection.ltr,
          child: Stack(
            children: [
              ModalBarrier(
                color: const Color(0xFF000000),
                onDismiss: () => dismissals++,
              ),
              GestureDetector(
                behavior: HitTestBehavior.translucent,
                onVerticalDragStart: (_) => drags++,
                child: const SizedBox.expand(),
              ),
            ],
          ),
        ),
      );
      const center = Offset(400, 300);
      final drag = await tester.startGesture(center);
      await drag.moveBy(const Offset(0, 65));
      await drag.up();
      expect((dismissals, drags), (0, 1));

      final canceled = await tester.startGesture(center);
      await canceled.cancel();
      expect((dismissals, drags), (0, 1));

      final mouse = await tester.startGesture(
        center,
        kind: PointerDeviceKind.mouse,
        buttons: kSecondaryMouseButton,
      );
      await mouse.moveBy(const Offset(0, 65));
      await mouse.up();
      expect((dismissals, drags), (0, 1));

      await tester.tapAt(center);
      expect((dismissals, drags), (1, 1));
    },
  );
}
