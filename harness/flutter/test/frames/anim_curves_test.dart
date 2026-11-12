import 'package:flutter/widgets.dart';

import 'recorder.dart';

void _addSample(FrameHarness h, String name, Curve curve, double t) {
  h.value(name, () => curve.transform(t));
}

void _addCurveSamples(FrameHarness h, String name, Curve curve) {
  for (final (suffix, t) in <(String, double)>[
    ('p0', 0.0),
    ('p21', 0.21),
    ('p37', 0.37),
    ('p63', 0.63),
    ('p79', 0.79),
    ('p100', 1.0),
  ]) {
    _addSample(h, '${name}_$suffix', curve, t);
  }
}

void main() {
  frameScenario('anim_curves__curves__interior', (h) async {
    final curves = <(String, Curve)>[
      ('linear', Curves.linear),
      ('decelerate', Curves.decelerate),
      ('fast_linear_to_slow_ease_in', Curves.fastLinearToSlowEaseIn),
      ('fast_ease_in_to_slow_ease_out', Curves.fastEaseInToSlowEaseOut),
      ('ease', Curves.ease),
      ('ease_in', Curves.easeIn),
      ('ease_in_to_linear', Curves.easeInToLinear),
      ('ease_in_sine', Curves.easeInSine),
      ('ease_in_quad', Curves.easeInQuad),
      ('ease_in_cubic', Curves.easeInCubic),
      ('ease_in_quart', Curves.easeInQuart),
      ('ease_in_quint', Curves.easeInQuint),
      ('ease_in_expo', Curves.easeInExpo),
      ('ease_in_circ', Curves.easeInCirc),
      ('ease_in_back', Curves.easeInBack),
      ('ease_out', Curves.easeOut),
      ('linear_to_ease_out', Curves.linearToEaseOut),
      ('ease_out_sine', Curves.easeOutSine),
      ('ease_out_quad', Curves.easeOutQuad),
      ('ease_out_cubic', Curves.easeOutCubic),
      ('ease_out_quart', Curves.easeOutQuart),
      ('ease_out_quint', Curves.easeOutQuint),
      ('ease_out_expo', Curves.easeOutExpo),
      ('ease_out_circ', Curves.easeOutCirc),
      ('ease_out_back', Curves.easeOutBack),
      ('ease_in_out', Curves.easeInOut),
      ('ease_in_out_sine', Curves.easeInOutSine),
      ('ease_in_out_quad', Curves.easeInOutQuad),
      ('ease_in_out_cubic', Curves.easeInOutCubic),
      ('ease_in_out_cubic_emphasized', Curves.easeInOutCubicEmphasized),
      ('ease_in_out_quart', Curves.easeInOutQuart),
      ('ease_in_out_quint', Curves.easeInOutQuint),
      ('ease_in_out_expo', Curves.easeInOutExpo),
      ('ease_in_out_circ', Curves.easeInOutCirc),
      ('ease_in_out_back', Curves.easeInOutBack),
      ('fast_out_slow_in', Curves.fastOutSlowIn),
      ('slow_middle', Curves.slowMiddle),
      ('bounce_in', Curves.bounceIn),
      ('bounce_out', Curves.bounceOut),
      ('bounce_in_out', Curves.bounceInOut),
      ('elastic_in', Curves.elasticIn),
      ('elastic_out', Curves.elasticOut),
      ('elastic_in_out', Curves.elasticInOut),
    ];
    for (final (name, curve) in curves) {
      _addCurveSamples(h, name, curve);
    }
    await h.mount((_) => const SizedBox.shrink());
  });

  frameScenario('anim_curves__curve_classes__edges', (h) async {
    _addCurveSamples(h, 'sawtooth_3', const SawTooth(3));
    _addCurveSamples(
      h,
      'interval_ease_in',
      const Interval(0.2, 0.8, curve: Curves.easeIn),
    );
    _addCurveSamples(
      h,
      'split_linear_ease_out_cubic',
      const Split(
        0.4,
        beginCurve: Curves.linear,
        endCurve: Curves.easeOutCubic,
      ),
    );
    _addCurveSamples(h, 'threshold_half', const Threshold(0.5));
    _addCurveSamples(h, 'cubic', const Cubic(0.2, 0.7, 0.8, 0.9));
    _addCurveSamples(
      h,
      'three_point_cubic',
      const ThreePointCubic(
        Offset(0.05, 0.0),
        Offset(0.13, 0.06),
        Offset(0.166666, 0.4),
        Offset(0.208333, 0.82),
        Offset(0.25, 1.0),
      ),
    );
    _addCurveSamples(h, 'flipped_ease_in', Curves.easeIn.flipped);
    _addCurveSamples(h, 'elastic_in_custom_period', const ElasticInCurve(0.6));
    _addCurveSamples(h, 'elastic_out_custom_period', const ElasticOutCurve(0.6));
    _addCurveSamples(h, 'elastic_in_out_custom_period', const ElasticInOutCurve(0.6));

    const interval = Interval(0.2, 0.8, curve: Curves.easeIn);
    const split = Split(
      0.4,
      beginCurve: Curves.linear,
      endCurve: Curves.easeOutCubic,
    );
    for (final (name, curve, t) in <(String, Curve, double)>[
      ('interval_before', interval, 0.1),
      ('interval_at_begin', interval, 0.2),
      ('interval_at_end', interval, 0.8),
      ('interval_after', interval, 0.9),
      ('split_at_split', split, 0.4),
      ('threshold_below', const Threshold(0.5), 0.499),
      ('threshold_equal', const Threshold(0.5), 0.5),
      ('threshold_above', const Threshold(0.5), 0.501),
      ('sawtooth_endpoint', const SawTooth(3), 1.0),
    ]) {
      _addSample(h, name, curve, t);
    }
    await h.mount((_) => const SizedBox.shrink());
  });

  frameScenario('anim_curves__curve2d__spline', (h) async {
    final points = <Offset>[
      const Offset(0.0, 0.0),
      const Offset(0.23, 0.16),
      const Offset(0.54, 0.82),
      const Offset(0.78, 0.31),
      const Offset(1.0, 1.0),
    ];
    final spline = CatmullRomSpline(points, tension: 0.2);
    for (final (suffix, t) in <(String, double)>[
      ('p13', 0.13),
      ('p37', 0.37),
      ('p68', 0.68),
      ('p91', 0.91),
    ]) {
      final point = spline.transform(t);
      h.value('spline_${suffix}_x', () => point.dx);
      h.value('spline_${suffix}_y', () => point.dy);
    }
    final samples = spline.generateSamples(tolerance: 1e-5).toList();
    h.value('spline_sample_count', () => samples.length.toDouble());
    for (final (name, sample) in <(String, Curve2DSample)>[
      ('spline_first_sample', samples.first),
      ('spline_middle_sample', samples[samples.length ~/ 2]),
      ('spline_last_sample', samples.last),
    ]) {
      h.value('${name}_t', () => sample.t);
      h.value('${name}_x', () => sample.value.dx);
      h.value('${name}_y', () => sample.value.dy);
    }
    h.value('spline_find_inverse_half', () => spline.findInverse(0.5));
    final handledSpline = CatmullRomSpline(
      points,
      tension: 0.2,
      startHandle: const Offset(-0.2, -0.1),
      endHandle: const Offset(1.2, 1.1),
    );
    final handledPoint = handledSpline.transform(0.5);
    h.value('spline_custom_handles_x', () => handledPoint.dx);
    h.value('spline_custom_handles_y', () => handledPoint.dy);

    final curvePoints = <Offset>[
      const Offset(0.2, 0.1),
      const Offset(0.5, 0.82),
      const Offset(0.8, 0.3),
    ];
    final curve = CatmullRomCurve(curvePoints, tension: 0.2);
    _addCurveSamples(h, 'catmull_rom_curve', curve);
    h.value('catmull_rom_curve_control_count', () => curve.controlPoints.length.toDouble());
    h.value('catmull_rom_curve_tension', () => curve.tension);
    h.value(
      'catmull_rom_curve_points_valid',
      () => CatmullRomCurve.validateControlPoints(curvePoints, tension: 0.2) ? 1.0 : 0.0,
    );
    await h.mount((_) => const SizedBox.shrink());
  });

  frameScenario('anim_curves__tweens__scalar', (h) async {
    final tween = Tween<double>(begin: 3.0, end: 13.0);
    final curveTween = CurveTween(curve: Curves.easeOutBack);
    final chained = tween.chain(curveTween);
    final reversed = ReverseTween<double>(tween);
    for (final (name, progress) in <(String, double)>[
      ('linear_start', 0.0),
      ('linear_quarter', 0.25),
      ('linear_middle', 0.5),
      ('linear_end', 1.0),
      ('linear_extrapolated', 1.5),
    ]) {
      h.value(name, () => tween.transform(progress));
    }
    for (final (name, progress) in <(String, double)>[
      ('curve_tween', 0.4),
      ('curve_tween_end', 1.0),
    ]) {
      h.value(name, () => curveTween.transform(progress));
    }
    h.value('chained_tween', () => chained.transform(0.4));
    for (final (name, progress) in <(String, double)>[
      ('reverse_tween', 0.25),
      ('reverse_tween_start', 0.0),
    ]) {
      h.value(name, () => reversed.transform(progress));
    }
    await h.mount((_) => const SizedBox.shrink());
  });

  frameScenario('anim_curves__tweens__typed', (h) async {
    final intTween = IntTween(begin: 2, end: 9);
    final stepTween = StepTween(begin: 2, end: 9);
    final constantTween = ConstantTween<double>(17.5);
    for (final (name, progress) in <(String, double)>[
      ('int_round_below_half', 0.35),
      ('int_round_half', 0.5),
      ('int_round_above_half', 0.65),
    ]) {
      h.value(name, () => intTween.transform(progress).toDouble());
    }
    for (final (name, progress) in <(String, double)>[
      ('step_below_half', 0.35),
      ('step_half', 0.5),
      ('step_at_end', 1.0),
    ]) {
      h.value(name, () => stepTween.transform(progress).toDouble());
    }
    for (final (name, progress) in <(String, double)>[
      ('constant_start', 0.0),
      ('constant_middle', 0.5),
      ('constant_end', 1.0),
    ]) {
      h.value(name, () => constantTween.transform(progress));
    }

    final colorTween = ColorTween(
      begin: const Color.fromARGB(40, 10, 80, 210),
      end: const Color.fromARGB(230, 210, 40, 90),
    );
    final color = colorTween.transform(0.37)!;
    h.value('color_red', () => (color.toARGB32() >> 16 & 0xff).toDouble());
    h.value('color_green', () => (color.toARGB32() >> 8 & 0xff).toDouble());
    h.value('color_blue', () => (color.toARGB32() & 0xff).toDouble());
    h.value('color_alpha', () => (color.toARGB32() >> 24 & 0xff).toDouble());

    final sizeTween = SizeTween(
      begin: const Size(10.0, 20.0),
      end: Size.zero,
    );
    final size = sizeTween.transform(0.37)!;
    h.value('size_width', () => size.width);
    h.value('size_height', () => size.height);
    final extrapolatedSize = sizeTween.transform(2.0)!;
    h.value('size_extrapolated_width', () => extrapolatedSize.width);
    h.value('size_extrapolated_height', () => extrapolatedSize.height);

    final rectTween = RectTween(
      begin: Rect.fromLTWH(0.0, 0.0, 10.0, 10.0),
      end: Rect.fromLTWH(100.0, 100.0, 0.0, 0.0),
    );
    final rect = rectTween.transform(0.37)!;
    h.value('rect_left', () => rect.left);
    h.value('rect_top', () => rect.top);
    h.value('rect_right', () => rect.right);
    h.value('rect_bottom', () => rect.bottom);
    final extrapolatedRect = rectTween.transform(2.0)!;
    h.value('rect_extrapolated_left', () => extrapolatedRect.left);
    h.value('rect_extrapolated_top', () => extrapolatedRect.top);
    h.value('rect_extrapolated_right', () => extrapolatedRect.right);
    h.value('rect_extrapolated_bottom', () => extrapolatedRect.bottom);
    await h.mount((_) => const SizedBox.shrink());
  });

  frameScenario('anim_curves__tween_sequence__weighted', (h) async {
    final items = <TweenSequenceItem<double>>[
      TweenSequenceItem<double>(tween: Tween<double>(begin: 0.0, end: 10.0), weight: 2.0),
      TweenSequenceItem<double>(tween: ConstantTween<double>(10.0), weight: 1.0),
      TweenSequenceItem<double>(tween: Tween<double>(begin: 10.0, end: 0.0), weight: 1.0),
    ];
    final sequence = TweenSequence<double>(items);
    final flipped = FlippedTweenSequence(items);
    for (final (name, progress) in <(String, double)>[
      ('sequence_start', 0.0),
      ('sequence_first_mid', 0.25),
      ('sequence_first_end', 0.5),
      ('sequence_constant_mid', 0.625),
      ('sequence_last_mid', 0.875),
      ('sequence_end', 1.0),
    ]) {
      h.value(name, () => sequence.transform(progress));
      h.value('flipped_$name', () => flipped.transform(progress));
    }
    final discontinuousItems = <TweenSequenceItem<double>>[
      TweenSequenceItem<double>(tween: Tween<double>(begin: 0.0, end: 0.2), weight: 2.0),
      TweenSequenceItem<double>(tween: ConstantTween<double>(0.7), weight: 1.0),
      TweenSequenceItem<double>(tween: Tween<double>(begin: 0.7, end: 1.0), weight: 1.0),
    ];
    final discontinuousSequence = TweenSequence<double>(discontinuousItems);
    final flippedDiscontinuousSequence = FlippedTweenSequence(discontinuousItems);
    h.value('sequence_boundary_discontinuous', () => discontinuousSequence.transform(0.5));
    h.value(
      'flipped_sequence_boundary_discontinuous',
      () => flippedDiscontinuousSequence.transform(0.5),
    );
    await h.mount((_) => const SizedBox.shrink());
  });
}
