import 'package:flutter/gestures.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  for (final button in [kPrimaryButton, kSecondaryButton, kTertiaryButton]) {
    testWidgets('long press reports start, long, end, up: $button', (
      tester,
    ) async {
      const key = ValueKey<String>('hold target');
      final trace = <String>[];
      await tester.pumpWidget(
        Directionality(
          textDirection: TextDirection.ltr,
          child: Center(
            child: GestureDetector(
              key: key,
              behavior: HitTestBehavior.opaque,
              onLongPressStart: (_) => trace.add('primary start'),
              onLongPress: () => trace.add('primary long'),
              onLongPressEnd: (_) => trace.add('primary end'),
              onLongPressUp: () => trace.add('primary up'),
              onSecondaryLongPressStart: (_) => trace.add('secondary start'),
              onSecondaryLongPress: () => trace.add('secondary long'),
              onSecondaryLongPressEnd: (_) => trace.add('secondary end'),
              onSecondaryLongPressUp: () => trace.add('secondary up'),
              onTertiaryLongPressStart: (_) => trace.add('tertiary start'),
              onTertiaryLongPress: () => trace.add('tertiary long'),
              onTertiaryLongPressEnd: (_) => trace.add('tertiary end'),
              onTertiaryLongPressUp: () => trace.add('tertiary up'),
              child: const SizedBox(width: 200, height: 200),
            ),
          ),
        ),
      );
      final contact = await tester.startGesture(
        tester.getCenter(find.byKey(key)),
        kind: PointerDeviceKind.mouse,
        buttons: button,
      );
      final family = switch (button) {
        kPrimaryButton => 'primary',
        kSecondaryButton => 'secondary',
        _ => 'tertiary',
      };
      await tester.pump(const Duration(milliseconds: 550));
      expect(trace, ['$family start', '$family long']);
      await contact.up();
      await contact.removePointer();
      expect(trace, [
        '$family start',
        '$family long',
        '$family end',
        '$family up',
      ]);
    });
  }
}
