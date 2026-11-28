import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  for (final behavior in HitTestBehavior.values) {
    testWidgets('empty drag source follows the configured hit policy ($behavior)', (
      WidgetTester tester,
    ) async {
      var starts = 0;
      var lowerTaps = 0;
      const feedback = ValueKey('feedback');
      const empty = SizedBox(width: 80, height: 50);
      await tester.pumpWidget(
        Directionality(
          textDirection: TextDirection.ltr,
          child: Overlay(initialEntries: [
            OverlayEntry(builder: (_) => Stack(children: [
              Positioned(
                left: 20, top: 20,
                child: GestureDetector(
                  behavior: HitTestBehavior.opaque,
                  onTap: () => lowerTaps++,
                  child: empty,
                ),
              ),
              Positioned(
                left: 20, top: 20,
                child: Draggable<int>(
                  data: 1,
                  hitTestBehavior: behavior,
                  onDragStarted: () => starts++,
                  feedback: const SizedBox(key: feedback, width: 20, height: 20),
                  child: empty,
                ),
              ),
            ])),
          ]),
        ),
      );
      final first = await tester.startGesture(const Offset(50, 40), pointer: 1);
      await tester.pump();
      expect(starts, behavior == HitTestBehavior.opaque ? 1 : 0);
      await first.up();
      await tester.pump();
      expect(lowerTaps, behavior == HitTestBehavior.opaque ? 0 : 1);

      if (behavior == HitTestBehavior.translucent) {
        final second = await tester.startGesture(const Offset(50, 40), pointer: 2);
        await second.moveBy(const Offset(0, 35));
        await tester.pump();
        expect(starts, 1);
        expect(find.byKey(feedback), findsOneWidget);
        await second.up();
        await tester.pump();
        expect(lowerTaps, 1);
      }
    });
  }
}
