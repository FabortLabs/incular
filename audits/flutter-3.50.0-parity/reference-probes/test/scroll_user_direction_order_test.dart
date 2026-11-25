import 'package:flutter/gestures.dart' show DragStartBehavior;
import 'package:flutter/rendering.dart' show ScrollDirection;
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

Widget traceList(ScrollPhysics physics, List<String> trace) {
  return Directionality(
    textDirection: TextDirection.ltr,
    child: NotificationListener<ScrollNotification>(
      onNotification: (notification) {
        switch (notification) {
          case UserScrollNotification(:final direction):
            trace.add(direction.name);
          case ScrollStartNotification():
            trace.add('start');
          case ScrollEndNotification():
            trace.add('end');
          case OverscrollNotification():
            trace.add('overscroll');
          case ScrollUpdateNotification():
            trace.add('update');
        }
        return false;
      },
      child: ListView.builder(
        physics: physics,
        dragStartBehavior: DragStartBehavior.down,
        itemExtent: 50,
        itemCount: 40,
        itemBuilder: (_, _) => const SizedBox.shrink(),
      ),
    ),
  );
}

void main() {
  testWidgets('clamped user direction precedes overscroll and changes once', (
    tester,
  ) async {
    final trace = <String>[];
    await tester.pumpWidget(traceList(const ClampingScrollPhysics(), trace));
    final gesture = await tester.startGesture(const Offset(400, 200));
    await gesture.moveBy(const Offset(0, 50));
    await tester.pump(const Duration(milliseconds: 16));
    await gesture.moveBy(const Offset(0, 50));
    await tester.pump(const Duration(milliseconds: 16));
    expect(trace.take(3), ['start', 'forward', 'overscroll']);
    expect(trace.where((e) => e == 'forward'), hasLength(1));
    await gesture.moveBy(const Offset(0, -50));
    await tester.pump(const Duration(milliseconds: 16));
    expect(trace.indexOf('reverse'), lessThan(trace.indexOf('update')));
    await tester.pump(const Duration(milliseconds: 100));
    await gesture.up();
    await tester.pumpAndSettle();
    expect(trace.where(ScrollDirection.values.map((d) => d.name).contains), [
      'forward',
      'reverse',
      'idle',
    ]);
    expect(trace.sublist(trace.length - 2), ['end', 'idle']);
  });

  testWidgets('flinging does not change direction until the activity ends', (
    tester,
  ) async {
    final trace = <String>[];
    await tester.pumpWidget(traceList(const BouncingScrollPhysics(), trace));
    await tester.fling(find.byType(ListView), const Offset(0, -160), 900);
    await tester.pump(const Duration(milliseconds: 16));
    expect(trace.take(3), ['start', 'reverse', 'update']);
    expect(trace, isNot(contains('idle')));
    await tester.pumpAndSettle();
    expect(trace.where(ScrollDirection.values.map((d) => d.name).contains), [
      'reverse',
      'idle',
    ]);
    expect(trace.sublist(trace.length - 2), ['end', 'idle']);
  });
}
