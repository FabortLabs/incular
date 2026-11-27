import 'package:flutter/gestures.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

Widget scene(
  List<String> events, {
  bool single = false,
  bool secondary = false,
  bool details = false,
}) {
  return Align(
    alignment: Alignment.topLeft,
    child: GestureDetector(
      behavior: HitTestBehavior.opaque,
      onTap: single ? () => events.add('single') : null,
      onTapDown: details ? (_) => events.add('down') : null,
      onTapCancel: details ? () => events.add('cancel') : null,
      onSecondaryTap: secondary ? () => events.add('secondary') : null,
      onDoubleTap: () => events.add('double'),
      child: const SizedBox(width: 200, height: 200),
    ),
  );
}

Future<void> tap(WidgetTester tester, int buttons, [Offset where = const Offset(80, 80)]) async {
  final gesture = await tester.startGesture(where, kind: PointerDeviceKind.mouse, buttons: buttons);
  await gesture.up();
  await tester.pump();
}

void main() {
  testWidgets('an in-route secondary press interrupts a mounted double tap', (tester) async {
    final events = <String>[];
    await tester.pumpWidget(scene(events));
    await tap(tester, kPrimaryMouseButton);
    await tester.pump(const Duration(milliseconds: 100));
    await tap(tester, kSecondaryMouseButton);
    await tester.pump(const Duration(milliseconds: 100));
    await tap(tester, kPrimaryMouseButton);
    expect(events, isEmpty);
    await tester.pump(const Duration(milliseconds: 70));
    await tap(tester, kPrimaryMouseButton);
    expect(events, ['double']);
    await tester.pump(const Duration(milliseconds: 301));
  });

  testWidgets('secondary interruption produces the mounted single-tap trace', (tester) async {
    final events = <String>[];
    await tester.pumpWidget(scene(events, single: true, secondary: true));
    await tap(tester, kPrimaryMouseButton);
    expect(events, isEmpty);
    await tester.pump(const Duration(milliseconds: 100));
    final gesture = await tester.startGesture(
      const Offset(80, 80),
      kind: PointerDeviceKind.mouse,
      buttons: kSecondaryMouseButton,
    );
    await tester.pump();
    final onDown = List<String>.of(events);
    await gesture.up();
    await tester.pump();
    final onUp = List<String>.of(events);
    await tester.pump(const Duration(milliseconds: 301));
    expect([onDown, onUp, events], [[], ['secondary'], ['secondary']]);
  });

  testWidgets('a secondary press without a callback leaves no phantom single', (tester) async {
    final events = <String>[];
    await tester.pumpWidget(scene(events, single: true));
    await tap(tester, kPrimaryMouseButton);
    expect(events, isEmpty);
    await tester.pump(const Duration(milliseconds: 100));
    final gesture = await tester.startGesture(
      const Offset(80, 80),
      kind: PointerDeviceKind.mouse,
      buttons: kSecondaryMouseButton,
    );
    await tester.pump();
    final onDown = List<String>.of(events);
    await gesture.up();
    await tester.pump();
    final onUp = List<String>.of(events);
    await tester.pump(const Duration(milliseconds: 301));
    expect([onDown, onUp, events], [[], [], []]);
  });

  testWidgets('rejecting an already reported primary tap cancels exactly once', (tester) async {
    final events = <String>[];
    await tester.pumpWidget(scene(events, single: true, details: true));
    final first = await tester.startGesture(
      const Offset(80, 80),
      kind: PointerDeviceKind.mouse,
      buttons: kPrimaryMouseButton,
    );
    await tester.pump(const Duration(milliseconds: 105));
    expect(events, ['down']);
    await first.up();
    await tester.pump();
    expect(events, ['down']);
    await tester.pump(const Duration(milliseconds: 100));
    await tap(tester, kSecondaryMouseButton);
    expect(events, ['down', 'cancel']);
    await tester.pump(const Duration(milliseconds: 301));
    expect(events, ['down', 'cancel']);
  });

  testWidgets('a secondary press off the detector does not interrupt the pair', (tester) async {
    final events = <String>[];
    await tester.pumpWidget(scene(events));
    await tap(tester, kPrimaryMouseButton);
    await tester.pump(const Duration(milliseconds: 100));
    await tap(tester, kSecondaryMouseButton, const Offset(220, 220));
    await tester.pump(const Duration(milliseconds: 100));
    await tap(tester, kPrimaryMouseButton);
    expect(events, ['double']);
    await tester.pump(const Duration(milliseconds: 301));
  });
}
