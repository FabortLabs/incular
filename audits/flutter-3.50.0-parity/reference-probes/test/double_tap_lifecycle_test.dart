import 'package:flutter/gestures.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

const point = Offset(80, 80);

Widget scene(List<String> events, {bool done = true}) {
  return GestureDetector(
    behavior: HitTestBehavior.opaque,
    onDoubleTapDown: (_) => events.add('down'),
    onDoubleTap: done ? () => events.add('double') : null,
    onDoubleTapCancel: () => events.add('cancel'),
    child: const SizedBox.expand(),
  );
}

Future<void> tap(WidgetTester tester) async {
  final gesture = await tester.startGesture(point);
  await gesture.up();
}

Future<void> gap(WidgetTester tester) => tester.pump(const Duration(milliseconds: 100));
Future<void> settle(WidgetTester tester) => tester.pump(const Duration(milliseconds: 400));

void main() {
  testWidgets('a moved first press cannot seed a later double tap', (tester) async {
    final trace = <String>[];
    await tester.pumpWidget(scene(trace));
    final first = await tester.startGesture(point);
    await first.moveBy(const Offset(22, 0));
    await first.up();
    await gap(tester);
    await tap(tester);
    expect(trace, isEmpty);
    await gap(tester);
    await tap(tester);
    expect(trace, ['down', 'double']);
    await settle(tester);
  });

  testWidgets('a moved second press cancels at the move and is not revived', (tester) async {
    final trace = <String>[];
    await tester.pumpWidget(scene(trace));
    await tap(tester);
    await gap(tester);
    final second = await tester.startGesture(point);
    expect(trace, ['down']);
    await second.moveBy(const Offset(22, 0));
    expect(trace, ['down', 'cancel']);
    await second.moveBy(const Offset(-22, 0));
    await second.up();
    await settle(tester);
    expect(trace, ['down', 'cancel']);
  });

  testWidgets('pointer cancellation is reported only for a qualified second press', (tester) async {
    final trace = <String>[];
    await tester.pumpWidget(scene(trace));
    final first = await tester.startGesture(point);
    await first.cancel();
    expect(trace, isEmpty);
    await gap(tester);
    await tap(tester);
    await gap(tester);
    final second = await tester.startGesture(point);
    expect(trace, ['down']);
    await second.cancel();
    expect(trace, ['down', 'cancel']);
    await settle(tester);
    expect(trace, ['down', 'cancel']);
  });

  testWidgets('down and cancel callbacks work without a completion callback', (tester) async {
    final trace = <String>[];
    await tester.pumpWidget(scene(trace, done: false));
    await tap(tester);
    await gap(tester);
    final second = await tester.startGesture(point);
    expect(trace, ['down']);
    await second.cancel();
    await settle(tester);
    expect(trace, ['down', 'cancel']);
  });
}
