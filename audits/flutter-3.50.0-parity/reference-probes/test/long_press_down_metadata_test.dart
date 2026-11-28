import 'package:flutter/gestures.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('long press down reports the initiating device and local origin', (
    tester,
  ) async {
    final seen = <(int, LongPressDownDetails)>[];
    await tester.pumpWidget(
      Directionality(
        textDirection: TextDirection.ltr,
        child: Padding(
          padding: const EdgeInsets.all(20),
          child: GestureDetector(
            behavior: HitTestBehavior.opaque,
            onLongPressDown: (d) => seen.add((kPrimaryButton, d)),
            onSecondaryLongPressDown: (d) => seen.add((kSecondaryButton, d)),
            onTertiaryLongPressDown: (d) => seen.add((kTertiaryButton, d)),
          ),
        ),
      ),
    );

    const global = Offset(55, 65);
    const cases = <(PointerDeviceKind, int)>[
      (PointerDeviceKind.touch, kPrimaryButton),
      (PointerDeviceKind.mouse, kSecondaryButton),
      (PointerDeviceKind.mouse, kTertiaryButton),
      (PointerDeviceKind.stylus, kPrimaryButton),
    ];
    for (var i = 0; i < cases.length; i++) {
      final (kind, button) = cases[i];
      final g = await tester.startGesture(global, kind: kind, buttons: button);
      expect(seen, hasLength(i + 1));
      final (observedButton, details) = seen.last;
      expect(observedButton, button);
      expect(details.kind, kind);
      expect(details.globalPosition, global);
      expect(details.localPosition, const Offset(35, 45));
      await g.up();
      expect(seen, hasLength(i + 1));
    }

    const defaults = LongPressDownDetails();
    expect(defaults.kind, isNull);
    expect(defaults.localPosition, Offset.zero);
  });
}
