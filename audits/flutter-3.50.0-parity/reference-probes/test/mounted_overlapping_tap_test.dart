import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  for (final bool reverse in <bool>[false, true]) {
    testWidgets(
      'only the first contact taps when release is ${reverse ? "FILO" : "FIFO"}',
      (tester) async {
        final positions = <Offset>[];
        await tester.pumpWidget(
          GestureDetector(
            behavior: HitTestBehavior.opaque,
            onTapUp: (details) => positions.add(details.globalPosition),
            child: const SizedBox.expand(),
          ),
        );
        final first = await tester.startGesture(const Offset(50, 50));
        final second = await tester.startGesture(const Offset(75, 50));
        expect(positions, isEmpty);
        if (reverse) {
          await second.up();
          expect(positions, isEmpty);
          await first.up();
        } else {
          await first.up();
          expect(positions, <Offset>[const Offset(50, 50)]);
          final third = await tester.startGesture(const Offset(90, 50));
          await third.up();
          await second.up();
        }
        expect(positions, <Offset>[const Offset(50, 50)]);
        await tester.tapAt(const Offset(100, 50));
        expect(positions, <Offset>[
          const Offset(50, 50),
          const Offset(100, 50),
        ]);
      },
    );
  }

  testWidgets(
    'canceling the primary does not promote a held secondary contact',
    (tester) async {
      final positions = <Offset>[];
      await tester.pumpWidget(
        GestureDetector(
          behavior: HitTestBehavior.opaque,
          onTapUp: (details) => positions.add(details.globalPosition),
          child: const SizedBox.expand(),
        ),
      );
      final first = await tester.startGesture(const Offset(50, 50));
      final second = await tester.startGesture(const Offset(75, 50));
      await first.cancel();
      await tester.tapAt(const Offset(90, 50));
      await second.up();
      expect(positions, isEmpty);

      await tester.tapAt(const Offset(100, 50));
      expect(positions, <Offset>[const Offset(100, 50)]);
    },
  );

  testWidgets('siblings may each recognize their own contact', (tester) async {
    final positions = <Offset>[];
    Widget pane() => Expanded(
      child: GestureDetector(
        behavior: HitTestBehavior.opaque,
        onTapUp: (details) => positions.add(details.globalPosition),
        child: const SizedBox.expand(),
      ),
    );
    await tester.pumpWidget(
      Directionality(
        textDirection: TextDirection.ltr,
        child: Row(children: <Widget>[pane(), pane()]),
      ),
    );
    final first = await tester.startGesture(const Offset(50, 50));
    final second = await tester.startGesture(const Offset(450, 50));
    await second.up();
    await first.up();
    expect(positions, <Offset>[const Offset(450, 50), const Offset(50, 50)]);
  });
}
