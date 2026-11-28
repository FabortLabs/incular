import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  for (final ignoring in [true, false]) {
    testWidgets('feedback accessibility is independent of pointer hits ($ignoring)', (
      WidgetTester tester,
    ) async {
      final semantics = tester.ensureSemantics();
      const source = ValueKey('source');
      const feedbackLabel = 'drag feedback';

      await tester.pumpWidget(
        Directionality(
          textDirection: TextDirection.ltr,
          child: Overlay(initialEntries: [
            OverlayEntry(builder: (_) => Center(
              child: Draggable<int>(
                data: 1,
                ignoringFeedbackSemantics: ignoring,
                feedback: Semantics(label: feedbackLabel, child: const SizedBox.square(dimension: 40)),
                child: const ColoredBox(
                  key: source, color: Color(0xff000000), child: SizedBox(width: 80, height: 50),
                ),
              ),
            )),
          ]),
        ),
      );

      final gesture = await tester.startGesture(tester.getCenter(find.byKey(source)));
      await tester.pump();
      expect(find.bySemanticsLabel(feedbackLabel), ignoring ? findsNothing : findsOneWidget);

      await gesture.up();
      await tester.pump();
      expect(find.bySemanticsLabel(feedbackLabel), findsNothing);
      semantics.dispose();
    });
  }
}
