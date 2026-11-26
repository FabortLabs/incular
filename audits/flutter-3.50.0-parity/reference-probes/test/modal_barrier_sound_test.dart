import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('locked barrier alerts once on release, never on press or cancel', (
    WidgetTester tester,
  ) async {
    final sounds = <Object?>[];
    final messenger = tester.binding.defaultBinaryMessenger;
    messenger.setMockMethodCallHandler(SystemChannels.platform, (call) async {
      if (call.method == 'SystemSound.play') {
        sounds.add(call.arguments);
      }
      return null;
    });
    try {
      for (final Widget barrier in [
        const ModalBarrier(dismissible: false),
        const AnimatedModalBarrier(
          color: AlwaysStoppedAnimation<Color?>(null),
          dismissible: false,
        ),
      ]) {
        sounds.clear();
        await tester.pumpWidget(barrier);
        const center = Offset(400, 300);
        final canceled = await tester.startGesture(center);
        expect(sounds, isEmpty);
        await canceled.cancel();
        expect(sounds, isEmpty);
        final tap = await tester.startGesture(center);
        expect(sounds, isEmpty);
        await tap.up();
        expect(sounds, ['SystemSoundType.alert']);
      }
    } finally {
      messenger.setMockMethodCallHandler(SystemChannels.platform, null);
    }
  });
}
