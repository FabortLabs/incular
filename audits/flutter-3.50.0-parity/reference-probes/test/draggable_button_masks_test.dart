import 'dart:ui' show PointerDeviceKind;

import 'package:flutter/gestures.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('drag source admits the full submitted mouse mask', (WidgetTester tester) async {
    for (final (filter, buttons, allowed) in <(bool Function(int)?, int, bool)>[
      (null, kPrimaryButton, true),
      (null, kSecondaryButton, false),
      (null, kPrimaryButton | kSecondaryButton, false),
      ((mask) => mask == kSecondaryButton, kPrimaryButton, false),
      ((mask) => mask == kSecondaryButton, kSecondaryButton, true),
      ((mask) => (mask & (kPrimaryButton | kTertiaryButton)) != 0, kTertiaryButton, true),
      ((mask) => (mask & (kPrimaryButton | kTertiaryButton)) != 0, kSecondaryButton | kTertiaryButton, true),
      ((mask) => false, kPrimaryButton, false),
    ]) {
      var starts = 0;
      const feedback = ValueKey('feedback');
      await tester.pumpWidget(
        Directionality(
          textDirection: TextDirection.ltr,
          child: Overlay(
            key: UniqueKey(),
            initialEntries: [
              OverlayEntry(builder: (_) => Center(
                child: Draggable<int>(
                  data: 1,
                  allowedButtonsFilter: filter,
                  onDragStarted: () => starts++,
                  feedback: const SizedBox(key: feedback, width: 20, height: 20),
                  child: const ColoredBox(
                    color: Color(0xff000000),
                    child: SizedBox(width: 80, height: 50),
                  ),
                ),
              )),
            ],
          ),
        ),
      );
      final gesture = await tester.startGesture(
        const Offset(400, 300), kind: PointerDeviceKind.mouse, buttons: buttons,
      );
      await tester.pump();
      expect(starts, allowed ? 1 : 0, reason: 'mouse buttons $buttons');
      expect(find.byKey(feedback), allowed ? findsOneWidget : findsNothing);
      await gesture.up();
      await tester.pump();
      expect(find.byKey(feedback), findsNothing);
    }
  });
}
