// Second batch of reference values: Catmull-Rom curves and splines,
// TickerFuture outcomes, and the strict end of an interpolation.
// Run: flutter test test/values/animation_values2_test.dart
import 'package:flutter/animation.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

String f(double v) => v.toStringAsFixed(9);

void main() {
  test('catmull rom', () {
    final samples = <double>[0.05, 0.1, 0.25, 0.4, 0.5, 0.6, 0.75, 0.9, 0.95];
    final curve = CatmullRomCurve(const <Offset>[
      Offset(0.2, 0.6),
      Offset(0.5, 0.3),
      Offset(0.8, 0.9),
    ]);
    print('CR0 ${samples.map((t) => f(curve.transform(t))).join(' ')}');
    final tense = CatmullRomCurve(const <Offset>[Offset(0.25, 0.1), Offset(0.6, 0.85)], tension: 0.4);
    print('CR1 ${samples.map((t) => f(tense.transform(t))).join(' ')}');
    final spline = CatmullRomSpline(const <Offset>[
      Offset(0, 0),
      Offset(0.3, 0.8),
      Offset(0.7, 0.2),
      Offset(1, 1),
    ]);
    print('SP ${samples.map((t) {
      final o = spline.transform(t);
      return '${f(o.dx)},${f(o.dy)}';
    }).join(' ')}');
    final generated = spline.generateSamples().toList();
    print('SPN ${generated.length}');
    print('SPS ${generated.take(6).map((s) => '${f(s.t)},${f(s.value.dx)},${f(s.value.dy)}').join(' ')}');
    print('INV ${f(spline.findInverse(0.5))}');
    print('VALID ${CatmullRomCurve.validateControlPoints(const <Offset>[Offset(0.5, 1.5), Offset(0.4, 0.2)])}');
  });

  testWidgets('ticker future', (tester) async {
    final controller = AnimationController(vsync: tester, duration: const Duration(milliseconds: 100));
    final log = <String>[];
    final first = controller.forward();
    first.whenCompleteOrCancel(() => log.add('first done-or-cancel'));
    first.then((_) => log.add('first complete'));
    first.orCancel.then((_) {}, onError: (e) => log.add('first orCancel ${e.runtimeType}'));
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 40));
    final second = controller.reverse();
    second.then((_) => log.add('second complete'));
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 40));
    print('STATUS@exact ${controller.status} ${controller.value}');
    await tester.pump(const Duration(milliseconds: 16));
    print('STATUS@after ${controller.status} ${controller.value}');
    await tester.pump();
    print('LOG ${log.join(' | ')}');
    controller.dispose();
  });
}
