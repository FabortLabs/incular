import 'package:flutter/gestures.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('mounted custom force interpolation sees native calibration', (
    tester,
  ) async {
    final samples = <(double, double, double)>[];
    await tester.pumpWidget(
      RawGestureDetector(
        behavior: HitTestBehavior.opaque,
        gestures: <Type, GestureRecognizerFactory>{
          ForcePressGestureRecognizer:
              GestureRecognizerFactoryWithHandlers<ForcePressGestureRecognizer>(
                () => ForcePressGestureRecognizer(
                  interpolation: (min, max, raw) {
                    samples.add((min, max, raw));
                    return (raw - min) / (max - min);
                  },
                ),
                (recognizer) => recognizer.onStart = (_) {},
              ),
        },
        child: const SizedBox.expand(),
      ),
    );
    tester.binding.handlePointerEvent(
      const PointerDownEvent(
        pointer: 1,
        position: Offset(20, 10),
        pressure: 0,
        pressureMin: 0,
        pressureMax: 10,
      ),
    );
    tester.binding.handlePointerEvent(
      const PointerMoveEvent(
        pointer: 1,
        position: Offset(21, 10),
        pressure: 4,
        pressureMin: 0,
        pressureMax: 10,
      ),
    );
    expect(samples, contains((0, 10, 4)));
    tester.binding.handlePointerEvent(
      const PointerUpEvent(pointer: 1, position: Offset(21, 10)),
    );
  });
}
