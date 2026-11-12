use incular_animation::{
    Animatable, AnimatableExt, CatmullRomCurve, CatmullRomSpline, ConstantTween, Curve, Curve2D,
    CurveTween, Curves, IntTween, LinearTween, RectTween, ReverseTween, SizeTween, StepTween,
    TweenSequence, TweenValue,
};
use incular_core::{Color, Offset, Rect, Size};
use incular_parity_harness::{FrameHarness, Scenario};
use incular_widgets::SizedBox;

fn add_sample(h: &mut FrameHarness, name: &str, curve: Curve, t: f32) {
    let name = name.to_owned();
    h.value(&name, move || f64::from(curve.apply(t)));
}

fn add_curve_samples(h: &mut FrameHarness, name: &str, curve: Curve) {
    for (suffix, t) in [
        ("p0", 0.0),
        ("p21", 0.21),
        ("p37", 0.37),
        ("p63", 0.63),
        ("p79", 0.79),
        ("p100", 1.0),
    ] {
        add_sample(h, &format!("{name}_{suffix}"), curve, t);
    }
}

fn mount(h: &mut FrameHarness) {
    h.mount(|_| SizedBox::shrink().into());
}

pub fn scenarios() -> Vec<Scenario> {
    vec![
        ("anim_curves__curves__interior", named_curves_interior),
        ("anim_curves__curve_classes__edges", curve_classes_edges),
        ("anim_curves__curve2d__spline", curve2d_spline),
        ("anim_curves__tweens__scalar", scalar_tweens),
        ("anim_curves__tweens__typed", typed_tweens),
        ("anim_curves__tween_sequence__weighted", weighted_sequence),
    ]
}

fn named_curves_interior(h: &mut FrameHarness) {
    let curves = [
        ("linear", Curves::LINEAR),
        ("decelerate", Curves::DECELERATE),
        (
            "fast_linear_to_slow_ease_in",
            Curves::FAST_LINEAR_TO_SLOW_EASE_IN,
        ),
        (
            "fast_ease_in_to_slow_ease_out",
            Curves::FAST_EASE_IN_TO_SLOW_EASE_OUT,
        ),
        ("ease", Curves::EASE),
        ("ease_in", Curves::EASE_IN),
        ("ease_in_to_linear", Curves::EASE_IN_TO_LINEAR),
        ("ease_in_sine", Curves::EASE_IN_SINE),
        ("ease_in_quad", Curves::EASE_IN_QUAD),
        ("ease_in_cubic", Curves::EASE_IN_CUBIC),
        ("ease_in_quart", Curves::EASE_IN_QUART),
        ("ease_in_quint", Curves::EASE_IN_QUINT),
        ("ease_in_expo", Curves::EASE_IN_EXPO),
        ("ease_in_circ", Curves::EASE_IN_CIRC),
        ("ease_in_back", Curves::EASE_IN_BACK),
        ("ease_out", Curves::EASE_OUT),
        ("linear_to_ease_out", Curves::LINEAR_TO_EASE_OUT),
        ("ease_out_sine", Curves::EASE_OUT_SINE),
        ("ease_out_quad", Curves::EASE_OUT_QUAD),
        ("ease_out_cubic", Curves::EASE_OUT_CUBIC),
        ("ease_out_quart", Curves::EASE_OUT_QUART),
        ("ease_out_quint", Curves::EASE_OUT_QUINT),
        ("ease_out_expo", Curves::EASE_OUT_EXPO),
        ("ease_out_circ", Curves::EASE_OUT_CIRC),
        ("ease_out_back", Curves::EASE_OUT_BACK),
        ("ease_in_out", Curves::EASE_IN_OUT),
        ("ease_in_out_sine", Curves::EASE_IN_OUT_SINE),
        ("ease_in_out_quad", Curves::EASE_IN_OUT_QUAD),
        ("ease_in_out_cubic", Curves::EASE_IN_OUT_CUBIC),
        (
            "ease_in_out_cubic_emphasized",
            Curves::EASE_IN_OUT_CUBIC_EMPHASIZED,
        ),
        ("ease_in_out_quart", Curves::EASE_IN_OUT_QUART),
        ("ease_in_out_quint", Curves::EASE_IN_OUT_QUINT),
        ("ease_in_out_expo", Curves::EASE_IN_OUT_EXPO),
        ("ease_in_out_circ", Curves::EASE_IN_OUT_CIRC),
        ("ease_in_out_back", Curves::EASE_IN_OUT_BACK),
        ("fast_out_slow_in", Curves::FAST_OUT_SLOW_IN),
        ("slow_middle", Curves::SLOW_MIDDLE),
        ("bounce_in", Curves::BOUNCE_IN),
        ("bounce_out", Curves::BOUNCE_OUT),
        ("bounce_in_out", Curves::BOUNCE_IN_OUT),
        ("elastic_in", Curves::ELASTIC_IN),
        ("elastic_out", Curves::ELASTIC_OUT),
        ("elastic_in_out", Curves::ELASTIC_IN_OUT),
    ];
    for (name, curve) in curves {
        add_curve_samples(h, name, curve);
    }
    mount(h);
}

fn curve_classes_edges(h: &mut FrameHarness) {
    static LINEAR: Curve = Curves::LINEAR;
    static EASE_IN: Curve = Curves::EASE_IN;
    static EASE_OUT_CUBIC: Curve = Curves::EASE_OUT_CUBIC;

    add_curve_samples(h, "sawtooth_3", Curve::SawTooth(3));
    add_curve_samples(h, "interval_ease_in", Curve::interval(0.2, 0.8, &EASE_IN));
    add_curve_samples(
        h,
        "split_linear_ease_out_cubic",
        Curve::split(0.4, &LINEAR, &EASE_OUT_CUBIC),
    );
    add_curve_samples(h, "threshold_half", Curve::Threshold(0.5));
    add_curve_samples(h, "cubic", Curve::cubic_bezier(0.2, 0.7, 0.8, 0.9));
    add_curve_samples(
        h,
        "three_point_cubic",
        Curve::ThreePointCubic {
            a1: (0.05, 0.0),
            b1: (0.13, 0.06),
            midpoint: (0.166666, 0.4),
            a2: (0.208333, 0.82),
            b2: (0.25, 1.0),
        },
    );
    add_curve_samples(h, "flipped_ease_in", Curves::EASE_IN.flipped());
    add_curve_samples(h, "elastic_in_custom_period", Curve::ElasticIn(0.6));
    add_curve_samples(h, "elastic_out_custom_period", Curve::ElasticOut(0.6));
    add_curve_samples(h, "elastic_in_out_custom_period", Curve::ElasticInOut(0.6));

    for (name, curve, t) in [
        ("interval_before", Curve::interval(0.2, 0.8, &EASE_IN), 0.1),
        (
            "interval_at_begin",
            Curve::interval(0.2, 0.8, &EASE_IN),
            0.2,
        ),
        ("interval_at_end", Curve::interval(0.2, 0.8, &EASE_IN), 0.8),
        ("interval_after", Curve::interval(0.2, 0.8, &EASE_IN), 0.9),
        (
            "split_at_split",
            Curve::split(0.4, &LINEAR, &EASE_OUT_CUBIC),
            0.4,
        ),
        ("threshold_below", Curve::Threshold(0.5), 0.499),
        ("threshold_equal", Curve::Threshold(0.5), 0.5),
        ("threshold_above", Curve::Threshold(0.5), 0.501),
        ("sawtooth_endpoint", Curve::SawTooth(3), 1.0),
    ] {
        add_sample(h, name, curve, t);
    }
    mount(h);
}

fn curve2d_spline(h: &mut FrameHarness) {
    let points = [
        Offset::new(0.0, 0.0),
        Offset::new(0.23, 0.16),
        Offset::new(0.54, 0.82),
        Offset::new(0.78, 0.31),
        Offset::new(1.0, 1.0),
    ];
    let spline = CatmullRomSpline::new(&points, 0.2);
    for (suffix, t) in [
        ("p13", 0.13_f64),
        ("p37", 0.37),
        ("p68", 0.68),
        ("p91", 0.91),
    ] {
        let (x, y) = spline.transform_xy(t);
        h.value(&format!("spline_{suffix}_x"), move || x);
        h.value(&format!("spline_{suffix}_y"), move || y);
    }
    let samples = spline.generate_samples(0.0, 1.0, 1e-5);
    h.value("spline_sample_count", {
        let count = samples.len();
        move || count as f64
    });
    for (name, sample) in [
        ("spline_first_sample", samples[0]),
        ("spline_middle_sample", samples[samples.len() / 2]),
        ("spline_last_sample", samples[samples.len() - 1]),
    ] {
        h.value(&format!("{name}_t"), move || sample.t);
        h.value(&format!("{name}_x"), move || sample.x);
        h.value(&format!("{name}_y"), move || sample.y);
    }
    h.value("spline_find_inverse_half", move || spline.find_inverse(0.5));

    let handled_spline = CatmullRomSpline::with_handles(
        &points,
        0.2,
        Some(Offset::new(-0.2, -0.1)),
        Some(Offset::new(1.2, 1.1)),
    );
    let (handled_x, handled_y) = handled_spline.transform_xy(0.5);
    h.value("spline_custom_handles_x", move || handled_x);
    h.value("spline_custom_handles_y", move || handled_y);

    let curve_points = [
        Offset::new(0.2, 0.1),
        Offset::new(0.5, 0.82),
        Offset::new(0.8, 0.3),
    ];
    let curve = CatmullRomCurve::new(&curve_points, 0.2);
    add_curve_samples(
        h,
        "catmull_rom_curve",
        Curve::catmull_rom(&curve_points, 0.2),
    );
    let control_count = curve.control_points().len();
    let curve_tension = curve.tension();
    h.value("catmull_rom_curve_control_count", move || {
        control_count as f64
    });
    h.value("catmull_rom_curve_tension", move || {
        f64::from(curve_tension)
    });
    let validation = CatmullRomCurve::validate_control_points(&curve_points, 0.2).is_ok();
    h.value("catmull_rom_curve_points_valid", move || {
        if validation { 1.0 } else { 0.0 }
    });
    mount(h);
}

fn scalar_tweens(h: &mut FrameHarness) {
    let tween = LinearTween {
        begin: 3.0_f32,
        end: 13.0_f32,
    };
    let curve_tween = CurveTween(Curves::EASE_OUT_BACK);
    let chained = tween.chain(curve_tween);
    let reversed = ReverseTween(tween);
    for (name, progress) in [
        ("linear_start", 0.0_f32),
        ("linear_quarter", 0.25),
        ("linear_middle", 0.5),
        ("linear_end", 1.0),
        ("linear_extrapolated", 1.5),
    ] {
        h.value(name, move || f64::from(tween.value_at(progress)));
    }
    for (name, progress) in [("curve_tween", 0.4_f32), ("curve_tween_end", 1.0)] {
        h.value(name, move || f64::from(curve_tween.value_at(progress)));
    }
    h.value("chained_tween", move || f64::from(chained.value_at(0.4)));
    for (name, progress) in [("reverse_tween", 0.25_f32), ("reverse_tween_start", 0.0)] {
        h.value(name, move || f64::from(reversed.value_at(progress)));
    }
    mount(h);
}

fn typed_tweens(h: &mut FrameHarness) {
    let int_tween = IntTween::new(2, 9);
    let step_tween = StepTween { begin: 2, end: 9 };
    let constant_tween = ConstantTween(17.5_f32);
    for (name, progress) in [
        ("int_round_below_half", 0.35_f32),
        ("int_round_half", 0.5),
        ("int_round_above_half", 0.65),
    ] {
        h.value(name, move || f64::from(int_tween.evaluate(progress)));
    }
    for (name, progress) in [
        ("step_below_half", 0.35_f32),
        ("step_half", 0.5),
        ("step_at_end", 1.0),
    ] {
        h.value(name, move || step_tween.value_at(progress) as f64);
    }
    for (name, progress) in [
        ("constant_start", 0.0_f32),
        ("constant_middle", 0.5),
        ("constant_end", 1.0),
    ] {
        h.value(name, move || f64::from(constant_tween.value_at(progress)));
    }

    let color_begin = Color::rgba(10, 80, 210, 40);
    let color_end = Color::rgba(210, 40, 90, 230);
    let color_tween = TweenValue::new(color_begin, color_end);
    let color = color_tween.evaluate(0.37);
    for (name, channel) in [
        ("color_red", color.red),
        ("color_green", color.green),
        ("color_blue", color.blue),
        ("color_alpha", color.alpha),
    ] {
        h.value(name, move || f64::from(channel));
    }

    let size_begin = Size::new(10.0, 20.0);
    let size_end = Size::new(0.0, 0.0);
    let size_tween = SizeTween::new(size_begin, size_end);
    let size = size_tween.evaluate(0.37);
    h.value("size_width", move || f64::from(size.width));
    h.value("size_height", move || f64::from(size.height));
    let extrapolated_size = size_tween.lerp(2.0);
    h.value("size_extrapolated_width", move || {
        f64::from(extrapolated_size.width)
    });
    h.value("size_extrapolated_height", move || {
        f64::from(extrapolated_size.height)
    });

    let rect_begin = Rect::from_origin_size(Offset::ZERO, Size::new(10.0, 10.0));
    let rect_end = Rect::from_origin_size(Offset::new(100.0, 100.0), Size::ZERO);
    let rect_tween = RectTween::new(rect_begin, rect_end);
    let rect = rect_tween.evaluate(0.37);
    h.value("rect_left", move || f64::from(rect.origin.x));
    h.value("rect_top", move || f64::from(rect.origin.y));
    h.value("rect_right", move || {
        f64::from(rect.origin.x + rect.size.width)
    });
    h.value("rect_bottom", move || {
        f64::from(rect.origin.y + rect.size.height)
    });
    let extrapolated_rect = rect_tween.transform(2.0);
    h.value("rect_extrapolated_left", move || {
        f64::from(extrapolated_rect.origin.x)
    });
    h.value("rect_extrapolated_top", move || {
        f64::from(extrapolated_rect.origin.y)
    });
    h.value("rect_extrapolated_right", move || {
        f64::from(extrapolated_rect.origin.x + extrapolated_rect.size.width)
    });
    h.value("rect_extrapolated_bottom", move || {
        f64::from(extrapolated_rect.origin.y + extrapolated_rect.size.height)
    });
    mount(h);
}

fn weighted_sequence(h: &mut FrameHarness) {
    let sequence = TweenSequence::new()
        .push(TweenValue::new(0.0_f32, 10.0), 2.0)
        .push(TweenValue::new(10.0, 10.0), 1.0)
        .push(TweenValue::new(10.0, 0.0), 1.0);
    for (name, progress) in [
        ("sequence_start", 0.0_f32),
        ("sequence_first_mid", 0.25),
        ("sequence_first_end", 0.5),
        ("sequence_constant_mid", 0.625),
        ("sequence_last_mid", 0.875),
        ("sequence_end", 1.0),
    ] {
        let sequence_for_value = sequence.clone();
        h.value(name, move || {
            f64::from(sequence_for_value.evaluate(progress).unwrap())
        });
        // Flutter's FlippedTweenSequence reverses the timeline and inverts the
        // normalized result. Incular has no named wrapper, so sample the source
        // formula explicitly as a diagnostic of that missing API.
        let sequence_for_flip = sequence.clone();
        h.value(&format!("flipped_{name}"), move || {
            1.0 - f64::from(sequence_for_flip.evaluate(1.0 - progress).unwrap())
        });
    }
    let discontinuous_sequence = TweenSequence::new()
        .push(TweenValue::new(0.0_f32, 0.2), 2.0)
        .push(TweenValue::new(0.7, 0.7), 1.0)
        .push(TweenValue::new(0.7, 1.0), 1.0);
    h.value("sequence_boundary_discontinuous", {
        let sequence = discontinuous_sequence.clone();
        move || f64::from(sequence.evaluate(0.5).unwrap())
    });
    h.value("flipped_sequence_boundary_discontinuous", move || {
        1.0 - f64::from(discontinuous_sequence.evaluate(0.5).unwrap())
    });
    mount(h);
}
