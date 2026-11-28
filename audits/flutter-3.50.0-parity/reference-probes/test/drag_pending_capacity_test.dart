import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('pending contacts recheck drag capacity when the arena accepts them', (
    WidgetTester tester,
  ) async {
    var starts = 0;
    const feedback = ValueKey('one feedback');
    final child = Row(mainAxisSize: MainAxisSize.min, children: [
      GestureDetector(behavior: HitTestBehavior.opaque, onTap: () {}, child: const SizedBox(width: 80, height: 50)),
      GestureDetector(behavior: HitTestBehavior.opaque, onTap: () {}, child: const SizedBox(width: 80, height: 50)),
    ]);
    await tester.pumpWidget(
      Directionality(
        textDirection: TextDirection.ltr,
        child: Overlay(initialEntries: [
          OverlayEntry(builder: (_) => Align(
            alignment: Alignment.topLeft,
            child: Draggable<int>(
              data: 1,
              maxSimultaneousDrags: 1,
              onDragStarted: () => starts++,
              feedback: const SizedBox(key: feedback, width: 20, height: 20),
              child: child,
            ),
          )),
        ]),
      ),
    );
    final first = await tester.startGesture(const Offset(20, 20), pointer: 1);
    final second = await tester.startGesture(const Offset(100, 20), pointer: 2);
    await tester.pump();
    expect(starts, 0);
    expect(find.byKey(feedback), findsNothing);

    await first.moveBy(const Offset(0, 35));
    await tester.pump();
    expect(starts, 1);
    expect(find.byKey(feedback), findsOneWidget);

    await second.moveBy(const Offset(0, 35));
    await tester.pump();
    expect(starts, 1);
    expect(find.byKey(feedback), findsOneWidget);

    await first.up();
    await tester.pump();
    expect(find.byKey(feedback), findsNothing);
    await second.moveBy(const Offset(0, 35));
    await tester.pump();
    expect(starts, 1);
    expect(find.byKey(feedback), findsNothing);
    await second.up();
  });
}
