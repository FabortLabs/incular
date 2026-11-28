import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('two local sources preserve independent feedback, candidates and payloads', (
    WidgetTester tester,
  ) async {
    final leaves = <int>[];
    final drops = <int>[];
    List<int?> candidates = [];

    Widget source(int data, double x) => Positioned(
      left: x,
      top: 20,
      child: Draggable<int>(
        data: data,
        feedback: SizedBox(key: ValueKey('feedback $data'), width: 40, height: 20),
        child: ColoredBox(
          key: ValueKey('source $data'),
          color: const Color(0xff101010),
          child: const SizedBox(width: 80, height: 40),
        ),
      ),
    );

    await tester.pumpWidget(
      Directionality(
        textDirection: TextDirection.ltr,
        child: Overlay(initialEntries: [
          OverlayEntry(builder: (_) => Stack(children: [
            source(7, 20),
            source(9, 120),
            Positioned(
              left: 20,
              top: 150,
              width: 300,
              height: 100,
              child: DragTarget<int>(
                onLeave: (value) => leaves.add(value!),
                onAcceptWithDetails: (details) => drops.add(details.data),
                builder: (_, accepted, _) {
                  candidates = accepted.toList();
                  return const SizedBox.expand();
                },
              ),
            ),
          ])),
        ]),
      ),
    );
    final first = await tester.startGesture(const Offset(60, 40), pointer: 1);
    await tester.pump();
    final second = await tester.startGesture(const Offset(160, 40), pointer: 2);
    await tester.pump();

    await first.moveTo(const Offset(90, 180));
    await second.moveTo(const Offset(170, 210));
    await tester.pump();
    expect(candidates, [7, 9]);
    expect(tester.getTopLeft(find.byKey(const ValueKey('feedback 7'))), const Offset(50, 160));
    expect(tester.getTopLeft(find.byKey(const ValueKey('feedback 9'))), const Offset(130, 190));

    await first.cancel();
    await tester.pump();
    expect(find.byKey(const ValueKey('feedback 7')), findsNothing);
    expect(find.byKey(const ValueKey('feedback 9')), findsOneWidget);
    expect(candidates, [9]);
    expect(leaves, [7]);
    expect(drops, isEmpty);

    await second.up();
    await tester.pump();
    expect(candidates, isEmpty);
    expect(drops, [9]);
    expect(leaves, [7]);
    expect(find.byKey(const ValueKey('feedback 9')), findsNothing);
  });
}
