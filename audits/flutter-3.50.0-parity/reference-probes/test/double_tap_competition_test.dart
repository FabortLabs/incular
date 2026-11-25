import 'package:flutter/gestures.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

Future<List<String>> mount(WidgetTester tester) async {
  final trace = <String>[];
  await tester.pumpWidget(
    Center(
      child: GestureDetector(
        behavior: HitTestBehavior.opaque,
        onTap: () => trace.add('single'),
        onDoubleTap: () => trace.add('double'),
        child: const SizedBox(width: 200, height: 200),
      ),
    ),
  );
  return trace;
}

const _center = Offset(400, 300);

Future<void> tap(
  WidgetTester tester, {
  Offset position = _center,
  int holdMs = 10,
}) async {
  final first = await tester.startGesture(position);
  await tester.pump(Duration(milliseconds: holdMs));
  await first.up();
}

void main() {
  testWidgets('a valid double tap never emits a single', (tester) async {
    final trace = await mount(tester);
    await tap(tester);
    expect(trace, isEmpty);
    await tester.pump(const Duration(milliseconds: 40));
    await tap(tester);
    await tester.pump(kDoubleTapTimeout);
    expect(trace, ['double']);
  });

  testWidgets('an unpaired tap wins exactly at the double-tap deadline', (
    tester,
  ) async {
    final trace = await mount(tester);
    await tap(tester);
    await tester.pump(kDoubleTapTimeout - const Duration(milliseconds: 1));
    expect(trace, isEmpty);
    await tester.pump(const Duration(milliseconds: 1));
    expect(trace, ['single']);
  });

  testWidgets('a qualifying second down pauses expiry until its release', (
    tester,
  ) async {
    final trace = await mount(tester);
    await tap(tester);
    await tester.pump(const Duration(milliseconds: 40));
    final second = await tester.startGesture(_center);
    await tester.pump(const Duration(milliseconds: 350));
    expect(trace, isEmpty);
    await second.up();
    expect(trace, ['double']);
  });

  for (final cancel in [true, false]) {
    testWidgets(
      'a rejected second contact does not restore the first: cancel=$cancel',
      (tester) async {
        final trace = await mount(tester);
        await tap(tester);
        await tester.pump(const Duration(milliseconds: 40));
        final second = await tester.startGesture(_center);
        await tester.pump(const Duration(milliseconds: 10));
        if (cancel) {
          await second.cancel();
        } else {
          await second.moveBy(const Offset(20, 0));
        }
        expect(trace, isEmpty);
        if (!cancel) {
          await second.up();
        }
        await tester.pump(kDoubleTapTimeout);
        expect(trace, isEmpty);
      },
    );
  }

  testWidgets('a far second tap is the only ordinary tap delivered', (
    tester,
  ) async {
    final trace = await mount(tester);
    await tap(tester);
    await tester.pump(const Duration(milliseconds: 40));
    await tap(tester, position: const Offset(305, 205));
    expect(trace, ['single']);
    await tester.pump(const Duration(milliseconds: 250));
    expect(trace, ['single']);
  });

  testWidgets('a too-early second press replaces the first wait', (
    tester,
  ) async {
    final trace = await mount(tester);
    await tap(tester, holdMs: 1);
    await tester.pump(const Duration(milliseconds: 4));
    final second = await tester.startGesture(_center);
    expect(trace, isEmpty);
    await tester.pump(const Duration(milliseconds: 5));
    await second.up();
    await tester.pump(const Duration(milliseconds: 299));
    expect(trace, isEmpty);
    await tester.pump(const Duration(milliseconds: 1));
    expect(trace, ['single']);
  });
}
