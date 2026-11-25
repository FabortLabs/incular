import 'package:flutter/scheduler.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  for (final restart in [false, true]) {
    testWidgets(
      'Ticker ${restart ? 'restart' : 'start'} waits until the next batch',
      (tester) async {
        final ticks = <Duration>[];
        final ticker = Ticker(ticks.add);
        addTearDown(ticker.dispose);
        tester.binding.scheduleFrameCallback((_) {
          if (restart) {
            ticker.stop();
          }
          ticker.start();
        });
        if (restart) {
          ticker.start();
        }
        await tester.pump();
        expect(ticks, isEmpty);
        await tester.pump(const Duration(milliseconds: 16));
        expect(ticks, [const Duration(milliseconds: 16)]);
      },
    );
  }
}
