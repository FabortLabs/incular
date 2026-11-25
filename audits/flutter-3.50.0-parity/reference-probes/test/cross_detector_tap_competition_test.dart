import 'package:flutter/gestures.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

Future<List<String>> mount(WidgetTester tester) async {
  final trace = <String>[];
  await tester.pumpWidget(
    Center(
      child: GestureDetector(
        behavior: HitTestBehavior.opaque,
        onTap: () => trace.add('parent single'),
        child: SizedBox(
          width: 200,
          height: 200,
          child: Align(
            alignment: Alignment.centerLeft,
            child: GestureDetector(
              behavior: HitTestBehavior.opaque,
              onDoubleTap: () => trace.add('child double'),
              child: const SizedBox(width: 100, height: 200),
            ),
          ),
        ),
      ),
    ),
  );
  return trace;
}

const _inside = Offset(390, 300);
const _outside = Offset(420, 300);

void main() {
  testWidgets('an inner double tap holds its ancestor single tap', (
    tester,
  ) async {
    final trace = await mount(tester);
    await tester.tapAt(_inside);
    expect(trace, isEmpty);
    await tester.pump(const Duration(milliseconds: 100));
    await tester.tapAt(_inside);
    await tester.pump(kDoubleTapTimeout);
    expect(trace, ['child double']);
  });
  testWidgets('a parent-only second press replaces its held first tap', (
    tester,
  ) async {
    final trace = await mount(tester);
    await tester.tapAt(_inside);
    expect(trace, isEmpty);
    await tester.pump(const Duration(milliseconds: 100));
    await tester.tapAt(_outside);
    expect(trace, ['parent single']);
    await tester.pump(kDoubleTapTimeout);
    expect(trace, ['parent single']);
  });
  testWidgets('without a second press the ancestor tap wins at expiry', (
    tester,
  ) async {
    final trace = await mount(tester);
    await tester.tapAt(_inside);
    expect(trace, isEmpty);
    await tester.pump(kDoubleTapTimeout);
    expect(trace, ['parent single']);
  });
}
