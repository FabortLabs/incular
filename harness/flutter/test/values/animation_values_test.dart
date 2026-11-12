// Prints reference values from Flutter's animation library for Incular's
// parity tests. Run: flutter test test/values/animation_values_test.dart
import 'package:flutter/animation.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

String f(double v) => v.toStringAsFixed(6);

void main() {
  test('curves', () {
    final samples = <double>[0.1, 0.25, 0.5, 0.75, 0.9];
    final curves = <String, Curve>{
      'decelerate': Curves.decelerate,
      'fastLinearToSlowEaseIn': Curves.fastLinearToSlowEaseIn,
      'fastEaseInToSlowEaseOut': Curves.fastEaseInToSlowEaseOut,
      'easeInToLinear': Curves.easeInToLinear,
      'linearToEaseOut': Curves.linearToEaseOut,
      'easeInOutCubicEmphasized': Curves.easeInOutCubicEmphasized,
      'slowMiddle': Curves.slowMiddle,
      'interval': const Interval(0.2, 0.7, curve: Curves.easeIn),
      'threshold': const Threshold(0.4),
      'sawTooth': const SawTooth(3),
      'split': Split(0.3),
      'easeFlipped': Curves.ease.flipped,
      'easeInSineFlipped': Curves.easeInSine.flipped,
      'elasticInFlipped': Curves.elasticIn.flipped,
      'elasticOut03': const ElasticOutCurve(0.3),
    };
    for (final entry in curves.entries) {
      print('CURVE ${entry.key} ${samples.map((t) => f(entry.value.transform(t))).join(' ')}');
    }
  });

  testWidgets('fling', (tester) async {
    final controller = AnimationController(vsync: tester);
    controller.fling(velocity: 2.0);
    final values = <String>[];
    var frames = 0;
    await tester.pump();
    while (controller.isAnimating && frames < 200) {
      values.add(f(controller.value));
      await tester.pump(const Duration(milliseconds: 16));
      frames++;
    }
    print('FLING frames=$frames status=${controller.status} last=${f(controller.value)}');
    print('FLING ${values.take(8).join(' ')}');
    controller.dispose();
  });

  testWidgets('repeat count', (tester) async {
    final controller = AnimationController(vsync: tester, duration: const Duration(milliseconds: 100), value: 0.25);
    controller.repeat(reverse: true, count: 2);
    final values = <String>[];
    final statuses = <String>[];
    await tester.pump();
    for (var i = 0; i < 14; i++) {
      values.add(f(controller.value));
      statuses.add(controller.status.name);
      await tester.pump(const Duration(milliseconds: 16));
    }
    print('REPEAT ${values.join(' ')}');
    print('REPEATSTATUS ${statuses.join(' ')}');
    controller.dispose();
  });

  testWidgets('animateTo below', (tester) async {
    final controller = AnimationController(vsync: tester, duration: const Duration(milliseconds: 100), value: 1.0);
    controller.animateTo(0.2);
    final statuses = <String>[controller.status.name];
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 50));
    statuses.add('${controller.status.name}@${f(controller.value)}');
    await tester.pump(const Duration(milliseconds: 60));
    statuses.add('${controller.status.name}@${f(controller.value)}');
    controller.animateBack(0.5, duration: const Duration(milliseconds: 40));
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 50));
    statuses.add('${controller.status.name}@${f(controller.value)}');
    print('ANIMATETO ${statuses.join(' ')}');
    controller.dispose();
  });

  testWidgets('curved animation latch', (tester) async {
    final controller = AnimationController(vsync: tester, duration: const Duration(milliseconds: 100));
    final curved = CurvedAnimation(parent: controller, curve: Curves.easeIn, reverseCurve: Curves.easeOut);
    controller.forward();
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 40));
    final a = curved.value;
    controller.reverse();
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 10));
    final b = curved.value;
    await tester.pump(const Duration(milliseconds: 100));
    controller.reverse(from: 1.0);
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 30));
    final c = curved.value;
    print('CURVED ${f(a)} ${f(b)} ${f(c)} parent=${f(controller.value)}');
    controller.dispose();
  });
}
