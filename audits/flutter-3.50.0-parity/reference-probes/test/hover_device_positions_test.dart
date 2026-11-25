import 'package:flutter/gestures.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

typedef MouseEvent = (String, String, int, Offset);

Widget scene(
  List<MouseEvent> events, {
  required bool swap,
  double width = 200,
}) {
  Positioned region(String name, double x, MouseCursor cursor) => Positioned(
    left: x,
    top: 0,
    child: MouseRegion(
      cursor: cursor,
      onEnter: (event) =>
          events.add((name, 'enter', event.device, event.position)),
      onHover: (event) =>
          events.add((name, 'hover', event.device, event.position)),
      onExit: (event) =>
          events.add((name, 'exit', event.device, event.position)),
      child: const SizedBox(width: 100, height: 100),
    ),
  );
  return Directionality(
    textDirection: TextDirection.ltr,
    child: Align(
      alignment: Alignment.topLeft,
      child: SizedBox(
        width: width,
        height: 100,
        child: Stack(
          children: [
            region('a', swap ? 100 : 0, SystemMouseCursors.text),
            region('b', swap ? 0 : 100, SystemMouseCursors.forbidden),
          ],
        ),
      ),
    ),
  );
}

TestGesture mouse(WidgetTester tester, int id) => TestGesture(
  dispatcher: tester.sendEventToBinding,
  pointer: id,
  device: id,
  kind: PointerDeviceKind.mouse,
);

void main() {
  testWidgets('touch does not move a mouse into the tapped region', (
    tester,
  ) async {
    final events = <MouseEvent>[];
    await tester.pumpWidget(scene(events, swap: false));
    final gesture = mouse(tester, 1);
    await gesture.addPointer(location: const Offset(50, 50));
    expect(events, [('a', 'enter', 1, const Offset(50, 50))]);
    events.clear();

    final tracker = RendererBinding.instance.mouseTracker;
    expect(tracker.debugDeviceActiveCursor(1), SystemMouseCursors.text);
    await tester.tapAt(const Offset(150, 50));
    expect(tracker.debugDeviceActiveCursor(1), SystemMouseCursors.text);
    await tester.pumpWidget(scene(events, swap: false, width: 220));
    expect(tracker.debugDeviceActiveCursor(1), SystemMouseCursors.text);
    expect(events, isEmpty);
    await gesture.removePointer();
  });

  testWidgets('layout refreshes every device at its own position', (
    tester,
  ) async {
    final events = <MouseEvent>[];
    await tester.pumpWidget(scene(events, swap: false));
    final first = mouse(tester, 1);
    final second = mouse(tester, 2);
    const left = Offset(50, 50);
    const right = Offset(150, 50);
    await first.addPointer(location: left);
    await second.addPointer(location: right);
    expect(events, [('a', 'enter', 1, left), ('b', 'enter', 2, right)]);
    events.clear();
    await tester.pumpWidget(scene(events, swap: true));
    expect(events, [
      ('a', 'exit', 1, left),
      ('b', 'enter', 1, left),
      ('b', 'exit', 2, right),
      ('a', 'enter', 2, right),
    ]);
    events.clear();
    await second.removePointer();
    expect(events, [('a', 'exit', 2, right)]);
    await first.removePointer();
  });
  testWidgets('a new pointer sequence on the same device does not reenter', (
    tester,
  ) async {
    final events = <MouseEvent>[];
    await tester.pumpWidget(scene(events, swap: false));
    final gesture = mouse(tester, 1);
    const initial = Offset(50, 50);
    const next = Offset(51, 50);
    await gesture.addPointer(location: initial);
    expect(events, [('a', 'enter', 1, initial)]);
    events.clear();
    await tester.sendEventToBinding(
      const PointerHoverEvent(
        pointer: 42,
        device: 1,
        kind: PointerDeviceKind.mouse,
        position: next,
      ),
    );
    expect(events, [('a', 'hover', 1, next)]);
    events.clear();
    await gesture.removePointer(location: next);
    expect(events, [('a', 'exit', 1, next)]);
  });
}
