import 'dart:ui' show PointerDeviceKind;

import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('a mouse drag survives replacement of its original hover child', (
    WidgetTester tester,
  ) async {
    const original = ValueKey<String>('old annotation');
    const replacement = ValueKey<String>('replacement');
    const feedback = ValueKey<String>('feedback');
    var starts = 0;
    var moves = 0;
    var oldHover = 0;
    var taps = 0;

    await tester.pumpWidget(
      Directionality(
        textDirection: TextDirection.ltr,
        child: Overlay(
          initialEntries: [
            OverlayEntry(
              builder: (_) => Center(
                child: Draggable<int>(
                  data: 1,
                  onDragStarted: () => starts++,
                  onDragUpdate: (_) => moves++,
                  feedback: const SizedBox(key: feedback, width: 20, height: 20),
                  childWhenDragging: const SizedBox(key: replacement, width: 20, height: 20),
                  child: MouseRegion(
                    key: original,
                    onHover: (_) => oldHover++,
                    child: GestureDetector(
                      behavior: HitTestBehavior.opaque,
                      onTap: () => taps++,
                      child: const SizedBox(width: 60, height: 20),
                    ),
                  ),
                ),
              ),
            ),
          ],
        ),
      ),
    );
    final gesture = await tester.startGesture(
      tester.getCenter(find.byKey(original)),
      kind: PointerDeviceKind.mouse,
    );
    await tester.pump(const Duration(seconds: 2));
    expect(starts, 0);
    expect(find.byKey(original), findsOneWidget);

    await gesture.moveBy(const Offset(10, 10));
    await tester.pump();
    expect(starts, 1);
    expect(find.byKey(original), findsNothing);
    expect(find.byKey(replacement), findsOneWidget);
    expect(find.byKey(feedback), findsOneWidget);

    final before = moves;
    final hoverBefore = oldHover;
    await gesture.moveBy(const Offset(10, 10));
    await tester.pump();
    expect(moves, greaterThan(before));
    expect(oldHover, hoverBefore);
    expect(find.byKey(replacement), findsOneWidget);
    await gesture.cancel();
    await tester.pump();
    expect(find.byKey(original), findsOneWidget);
    expect(find.byKey(replacement), findsNothing);
    expect(find.byKey(feedback), findsNothing);
    expect(taps, 0);
  });
}
