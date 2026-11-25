use incular_core::{Offset, Transform};
use incular_gestures::{BoxHitTestResult, HitTestResult, HitTestTransform};

const POS: Offset = Offset { x: 3., y: 4. };
const TRANSLATE: Offset = Offset { x: 20., y: 30. };

#[test]
fn a_box_view_appends_to_the_original_path_and_keeps_the_current_transform() {
    let mut result = HitTestResult::new();
    let transform = Transform::translation(Offset::new(40., 150.));
    result.push_transform(transform);
    result.add(1);
    {
        let mut view = BoxHitTestResult::wrap(&mut result);
        assert_eq!(view.path()[0].target, 1);
        view.add(2);
    }
    result.add(3);
    let mut view = BoxHitTestResult::wrap(&mut result);
    assert_eq!(
        view.path()
            .iter()
            .map(|entry| entry.target)
            .collect::<Vec<_>>(),
        [1, 2, 3]
    );
    assert!(view.path().iter().all(|entry| entry.transform == transform));
    view.add_with_paint_transform(Some(Transform::scale(2.)), POS, |view, _| {
        view.add(4);
        false
    });
    view.add(5);
    assert_eq!(
        view.path()[3].transform,
        Transform::scale(0.5).then(transform)
    );
    assert_eq!(view.path()[4].transform, transform);
}

#[test]
fn paint_transform_and_offset_map_the_same_points_and_return_the_delegate_result() {
    let mut target = HitTestResult::<()>::new();
    let mut result = BoxHitTestResult::wrap(&mut target);
    for (offset, original, mapped, accepted) in [
        (None, Offset::ZERO, Offset::ZERO, true),
        (
            Some(Offset::new(55., 32.)),
            Offset::ZERO,
            Offset::new(-55., -32.),
            true,
        ),
        (None, POS, POS, false),
        (Some(Offset::ZERO), POS, POS, true),
        (Some(TRANSLATE), POS, POS - TRANSLATE, true),
    ] {
        for with_matrix in [false, true] {
            let mut seen = None;
            let observe = |_: &mut BoxHitTestResult<'_, ()>, point| {
                seen = Some(point);
                accepted
            };
            let hit = if with_matrix {
                result.add_with_paint_transform(
                    offset.map(Transform::translation),
                    original,
                    observe,
                )
            } else {
                result.add_with_paint_offset(offset, original, observe)
            };
            assert_eq!(hit, accepted);
            assert_eq!(seen, Some(mapped));
        }
    }
    assert!(
        !result.add_with_paint_transform(Some(Transform::scale(0.)), POS, |_, _| {
            panic!("a singular paint transform must not call the delegate")
        })
    );
}

#[test]
fn raw_transforms_apply_forward_instead_of_inverting() {
    let mut target = HitTestResult::<()>::new();
    let mut result = BoxHitTestResult::wrap(&mut target);
    for (transform, input, expected, accepted) in [
        (None, Offset::ZERO, Offset::ZERO, true),
        (
            Some(Transform::translation(TRANSLATE)),
            Offset::ZERO,
            TRANSLATE,
            true,
        ),
        (None, POS, POS, false),
        (Some(Transform::IDENTITY), POS, POS, true),
        (
            Some(Transform::translation(TRANSLATE)),
            POS,
            POS + TRANSLATE,
            true,
        ),
    ] {
        let mut seen = None;
        assert_eq!(
            result.add_with_raw_transform(transform, input, |_, point| {
                seen = Some(point);
                accepted
            }),
            accepted
        );
        assert_eq!(seen, Some(expected));
    }
}

#[test]
fn out_of_band_hit_scopes_the_transform_and_accepts_singular_raw_transforms() {
    let mut target = HitTestResult::new();
    let mut result = BoxHitTestResult::wrap(&mut target);
    let matrix = Transform::translation(TRANSLATE);
    for (kind, expected) in [
        (
            HitTestTransform::PaintOffset(TRANSLATE),
            matrix.inverse().unwrap(),
        ),
        (HitTestTransform::Paint(matrix), matrix.inverse().unwrap()),
        (HitTestTransform::Raw(matrix), matrix),
        (
            HitTestTransform::Raw(Transform::scale(0.)),
            Transform::scale(0.),
        ),
    ] {
        assert!(result.add_with_out_of_band_position(kind, |view| {
            view.add(1);
            true
        }));
        assert_eq!(result.path().last().unwrap().transform, expected);
    }
    result.add(2);
    assert_eq!(result.path().last().unwrap().transform, Transform::IDENTITY);
    if cfg!(debug_assertions) {
        let ran = std::cell::Cell::new(false);
        let failure = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            result.add_with_out_of_band_position(
                HitTestTransform::Paint(Transform::scale(0.)),
                |_| {
                    ran.set(true);
                    true
                },
            )
        }))
        .unwrap_err();
        assert!(!ran.get());
        assert_eq!(
            failure.downcast_ref::<&str>(),
            Some(&"paintTransform must be invertible.")
        );
    }
}
