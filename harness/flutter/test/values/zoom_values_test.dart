import 'package:flutter/animation.dart';
import 'package:flutter_test/flutter_test.dart';

String f(double v) => v.toStringAsFixed(6);

void main() {
  test('zoom', () {
    final seq = TweenSequence<double>(<TweenSequenceItem<double>>[
      TweenSequenceItem<double>(
        tween: Tween<double>(begin: 0.0, end: 0.4).chain(CurveTween(curve: const Cubic(0.05, 0.0, 0.133333, 0.06))),
        weight: 0.166666,
      ),
      TweenSequenceItem<double>(
        tween: Tween<double>(begin: 0.4, end: 1.0).chain(CurveTween(curve: const Cubic(0.208333, 0.82, 0.25, 1.0))),
        weight: 1.0 - 0.166666,
      ),
    ]);
    final samples = <double>[0.05, 0.1, 0.166666, 0.25, 0.5, 0.75, 0.95];
    print('ZSEQ ${samples.map((t) => f(seq.transform(t))).join(' ')}');
    print('FFWD ${samples.map((t) => f(Curves.easeInOutCubicEmphasized.transform(t))).join(' ')}');
  });
}
