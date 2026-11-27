use incular_core::{Offset, Transform};
use incular_gestures::{HitTestResult, SharedHitTestResult};

fn transform_of_next(result: &SharedHitTestResult<usize>) -> Transform {
    let next = result.path().len();
    result.add(next);
    result.path().last().unwrap().transform
}

fn compare(actual: Transform, expected: Transform) {
    for (actual, expected) in actual
        .coefficients()
        .into_iter()
        .zip(expected.coefficients())
    {
        assert!((actual - expected).abs() < 1e-10, "{actual} != {expected}");
    }
}

#[test]
fn wrappers_share_the_same_path_and_inherit_its_current_transform() {
    let shift = Transform::translation(Offset::new(40., 150.));
    let wrapped = HitTestResult::new().into_shared();
    wrapped.push_transform(shift);
    wrapped.add(1);
    assert_eq!(wrapped.path()[0].target, 1);
    assert_eq!(wrapped.path()[0].transform, shift);

    let wrapping = SharedHitTestResult::wrap(&wrapped);
    assert!(wrapping.shares_path_with(&wrapped));
    assert_eq!(wrapping.path().as_ptr(), wrapped.path().as_ptr());
    wrapping.add(2);
    assert_eq!(wrapped.path()[1].target, 2);
    assert_eq!(wrapped.path()[1].transform, shift);
    wrapped.add(3);
    for path in [wrapped.path(), wrapping.path()] {
        assert_eq!(
            path.iter().map(|hit| hit.target).collect::<Vec<_>>(),
            [1, 2, 3]
        );
        assert!(path.iter().all(|hit| hit.transform == shift));
    }
}

#[test]
fn wrappers_share_each_transform_push_pop_and_the_path_snapshots() {
    let result = HitTestResult::new().into_shared();
    let translate = Transform::translation(Offset::new(10., 20.));
    let rotate = Transform::rotation(1.);
    let scale = Transform::scale_non_uniform(1.1, 1.2);
    result.push_transform(translate);
    compare(transform_of_next(&result), translate);
    result.push_transform(rotate);
    for _ in 0..2 {
        compare(transform_of_next(&result), rotate.then(translate));
    }
    let wrapped = SharedHitTestResult::wrap(&result);
    compare(transform_of_next(&wrapped), rotate.then(translate));

    result.push_transform(scale);
    for r in [&result, &wrapped] {
        let transform = transform_of_next(r);
        compare(transform, scale.then(rotate).then(translate));
        let point = transform.transform_point(Offset::new(2., 3.));
        let (sin, cos) = 1_f32.sin_cos();
        let expected = Offset::new((12. * cos - 23. * sin) * 1.1, (12. * sin + 23. * cos) * 1.2);
        assert!((point.x - expected.x).abs() < 0.00001);
        assert!((point.y - expected.y).abs() < 0.00001);
    }

    wrapped.pop_transform();
    result.pop_transform();
    compare(transform_of_next(&result), translate);
    result.pop_transform();
    result.push_transform(scale);
    compare(transform_of_next(&wrapped), scale);
    result.push_transform(rotate);
    compare(transform_of_next(&result), rotate.then(scale));
    assert_eq!(result.path()[0].transform, translate);
}

#[test]
fn offset_order_is_forward_and_interleaves_with_other_transforms() {
    let result = HitTestResult::new().into_shared();
    let offset = Offset::new(10., 20.);
    let translate = Transform::translation(offset);
    let rotate = Transform::rotation(1.);
    let scale = Transform::scale_non_uniform(1.1, 1.2);

    result.push_offset(offset);
    compare(transform_of_next(&result), translate);
    result.pop_transform();

    result.push_offset(offset);
    result.push_transform(rotate);
    for _ in 0..2 {
        compare(transform_of_next(&result), rotate.then(translate));
    }
    let wrapped = SharedHitTestResult::wrap(&result);
    compare(transform_of_next(&wrapped), rotate.then(translate));

    result.push_transform(scale);
    for r in [&result, &wrapped] {
        compare(transform_of_next(r), scale.then(rotate).then(translate));
    }
    result.pop_transform();
    result.pop_transform();
    wrapped.pop_transform();
    compare(transform_of_next(&result), Transform::IDENTITY);

    result.push_transform(scale);
    wrapped.push_offset(offset);
    result.push_transform(rotate);
    compare(
        transform_of_next(&result),
        rotate.then(translate).then(scale),
    );
    wrapped.pop_transform();
    compare(transform_of_next(&result), translate.then(scale));
}
