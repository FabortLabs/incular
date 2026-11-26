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
}
