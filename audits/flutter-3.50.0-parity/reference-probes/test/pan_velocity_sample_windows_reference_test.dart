import 'package:flutter/gestures.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

Future<Offset> releaseVelocity(
  WidgetTester tester, {
  required int sampleGap,
  required int releaseDelay,
}) async {
  Offset? velocity;
  await tester.pumpWidget(
    Directionality(
      textDirection: TextDirection.ltr,
      child: GestureDetector(
        behavior: HitTestBehavior.opaque,
        onPanEnd: (details) => velocity = details.velocity.pixelsPerSecond,
        child: const SizedBox.expand(),
      ),
    ),
  );

  final touch = await tester.createGesture(kind: PointerDeviceKind.touch);
  await touch.down(const Offset(250, 250), timeStamp: Duration.zero);
  for (final (ms, position) in [
    (10, const Offset(260, 255)),
    (10 + sampleGap, const Offset(100, 150)),
    (20 + sampleGap, const Offset(107, 189)),
    (30 + sampleGap, const Offset(114, 228)),
  ]) {
    await touch.moveTo(position, timeStamp: Duration(milliseconds: ms));
  }
  if (releaseDelay != 0) {
    await tester.pump(Duration(milliseconds: releaseDelay));
  }
  await touch.up(
    timeStamp: Duration(milliseconds: 30 + sampleGap + releaseDelay),
  );
  expect(velocity, isNotNull, reason: 'the pan ended');
  return velocity!;
}

void expectVelocity(Offset actual, Offset expected) {
  expect(actual.dx, closeTo(expected.dx, 0.01));
  expect(actual.dy, closeTo(expected.dy, 0.01));
}

void main() {
  testWidgets('A gap over 40 ms excludes older samples', (tester) async {
    expectVelocity(
      await releaseVelocity(tester, sampleGap: 40, releaseDelay: 0),
      const Offset(-39.09806125316795, 3842.778870469235),
    );
    expectVelocity(
      await releaseVelocity(tester, sampleGap: 41, releaseDelay: 0),
      const Offset(700, 3900),
    );
  });

  testWidgets('A release after more than 40 ms reports zero', (tester) async {
    expectVelocity(
      await releaseVelocity(tester, sampleGap: 41, releaseDelay: 40),
      const Offset(700, 3900),
    );
    expect(
      await releaseVelocity(tester, sampleGap: 41, releaseDelay: 41),
      Offset.zero,
    );
  });
}
