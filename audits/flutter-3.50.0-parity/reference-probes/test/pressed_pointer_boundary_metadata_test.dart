import 'package:flutter/gestures.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets(
    'Pressed stylus boundary events zero pressure but retain contact',
    (WidgetTester tester) async {
      final events = <PointerEvent>[];
      await tester.pumpWidget(
        Directionality(
          textDirection: TextDirection.ltr,
          child: Stack(
            children: [
              Positioned(
                left: 100,
                top: 100,
                child: MouseRegion(
                  onEnter: events.add,
                  onExit: events.add,
                  child: const SizedBox(width: 100, height: 100),
                ),
              ),
            ],
          ),
        ),
      );
      await tester.sendEventToBinding(
        const PointerAddedEvent(
          kind: PointerDeviceKind.stylus,
          device: 10,
          position: Offset(50, 150),
        ),
      );
      await tester.sendEventToBinding(
        const PointerDownEvent(
          pointer: 34,
          kind: PointerDeviceKind.stylus,
          device: 10,
          position: Offset(60, 150),
          pressure: 0.5,
        ),
      );
      await tester.sendEventToBinding(
        const PointerMoveEvent(
          pointer: 34,
          kind: PointerDeviceKind.stylus,
          device: 10,
          position: Offset(130, 150),
          delta: Offset(70, 0),
          buttons: kPrimaryButton,
          pressure: 0.55,
          orientation: 1.1,
          tilt: 0.8,
        ),
      );
      await tester.sendEventToBinding(
        const PointerMoveEvent(
          pointer: 34,
          kind: PointerDeviceKind.stylus,
          device: 10,
          position: Offset(220, 150),
          delta: Offset(90, 0),
          buttons: kPrimaryButton,
          pressure: 0.7,
          orientation: 1.1,
          tilt: 0.8,
        ),
      );
      expect(events, [isA<PointerEnterEvent>(), isA<PointerExitEvent>()]);
      for (final event in events) {
        expect(event.pointer, 34);
        expect(event.device, 10);
        expect(event.kind, PointerDeviceKind.stylus);
        expect(event.buttons, kPrimaryButton);
        expect(event.down, isTrue);
        expect(event.pressure, 0);
        expect(event.orientation, 1.1);
        expect(event.tilt, 0.8);
      }
      expect(events[0].localPosition, const Offset(30, 50));
      expect(events[0].localDelta, const Offset(70, 0));
      expect(events[1].localPosition, const Offset(120, 50));
      expect(events[1].localDelta, const Offset(90, 0));
    },
  );
}
