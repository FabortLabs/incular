//! Flutter's `DisplayFeatureSubScreen` and display features.

use std::{cell::RefCell, rc::Rc};

use incular_config::{
    Constraints, DisplayFeature, DisplayFeatureState, DisplayFeatureType, EdgeInsets,
    RuntimeEnvironment, TextDirection,
};
use incular_core::{Color, Offset, Rect, Size, Transform};
use incular_rendering::{DisplayList, PaintCommand};
use incular_widgets::internal::WidgetTree;
use incular_widgets::{Builder, Directionality, DisplayFeatureSubScreen, Widget};

const VIEW: Size = Size {
    width: 400.,
    height: 200.,
};

fn rect(left: f32, top: f32, right: f32, bottom: f32) -> Rect {
    Rect::from_origin_size(
        Offset::new(left, top),
        Size::new(right - left, bottom - top),
    )
}

fn hinge() -> DisplayFeature {
    DisplayFeature {
        bounds: rect(190., 0., 210., 200.),
        kind: DisplayFeatureType::Hinge,
        state: DisplayFeatureState::PostureFlat,
    }
}

fn painted_rects(list: &DisplayList) -> Vec<Rect> {
    let mut transforms = vec![Transform::IDENTITY];
    let mut rects = Vec::new();
    for command in list.commands() {
        match command {
            PaintCommand::PushTransform { transform } => {
                let current = *transforms.last().unwrap();
                transforms.push(current.then(*transform));
            }
            PaintCommand::PopTransform => {
                transforms.pop();
            }
            PaintCommand::Rect { rect, .. } => {
                rects.push(transforms.last().unwrap().transform_rect_bbox(*rect));
            }
            _ => {}
        }
    }
    rects
}

struct Seen {
    features: Vec<DisplayFeature>,
    safe_insets: EdgeInsets,
}

/// Lays out `screen` in a hinged view and reports where its child painted
/// and what the child saw.
fn place(
    direction: TextDirection,
    screen: impl FnOnce(Widget) -> DisplayFeatureSubScreen,
) -> (Rect, Seen) {
    let seen = Rc::new(RefCell::new(None));
    let child = {
        let seen = seen.clone();
        Builder::new(move |context| {
            *seen.borrow_mut() = Some(Seen {
                features: context.display_features(),
                safe_insets: context.safe_insets(),
            });
            Widget::box_(Size::new(10., 10.), Color::WHITE)
        })
    };
    let mut tree = WidgetTree::new();
    tree.set_environment(RuntimeEnvironment {
        viewport: VIEW,
        safe_insets: EdgeInsets {
            left: 12.,
            top: 0.,
            right: 8.,
            bottom: 0.,
        },
        display_features: vec![hinge()],
        ..RuntimeEnvironment::default()
    });
    tree.mount(Directionality::new(direction, screen(child.into())).into())
        .expect("mount");
    tree.layout(Constraints::tight(VIEW)).expect("layout");
    let rects = painted_rects(&tree.paint());
    let seen = seen.borrow_mut().take().expect("built");
    (rects[0], seen)
}

#[test]
fn the_start_sub_screen_holds_the_child_without_an_anchor() {
    let (left, seen) = place(TextDirection::Ltr, DisplayFeatureSubScreen::new);
    assert_eq!(left.origin, Offset::ZERO);
    // The hinge is outside the left sub-screen, and the right inset is
    // covered by the 210 px to its right.
    assert!(seen.features.is_empty());
    assert_eq!(
        seen.safe_insets,
        EdgeInsets {
            left: 12.,
            top: 0.,
            right: 0.,
            bottom: 0.
        }
    );

    let (right, seen) = place(TextDirection::Rtl, DisplayFeatureSubScreen::new);
    assert_eq!(right.origin, Offset::new(210., 0.));
    // max(0, 12 - 210) and max(0, 8 - 0).
    assert_eq!(
        seen.safe_insets,
        EdgeInsets {
            left: 0.,
            top: 0.,
            right: 8.,
            bottom: 0.
        }
    );
}

#[test]
fn an_anchor_picks_the_closest_sub_screen() {
    let (placed, _) = place(TextDirection::Ltr, |child| {
        DisplayFeatureSubScreen::new(child).anchor_point(Offset::new(205., 50.))
    });
    // 205 is 5 from the right sub-screen and 15 from the left one.
    assert_eq!(placed.origin, Offset::new(210., 0.));
    // An anchor outside the view is capped into it first.
    let (placed, _) = place(TextDirection::Ltr, |child| {
        DisplayFeatureSubScreen::new(child).anchor_point(Offset::new(-50., 500.))
    });
    assert_eq!(placed.origin, Offset::ZERO);
}

#[test]
fn sub_screens_split_around_each_avoided_bound() {
    let view = rect(0., 0., 400., 200.);
    assert_eq!(
        DisplayFeatureSubScreen::sub_screens_in_bounds(view, &[rect(190., 0., 210., 200.)]),
        [rect(0., 0., 190., 200.), rect(210., 0., 400., 200.)]
    );
    // A cutout that does not span the current panel is not a separator.
    assert_eq!(
        DisplayFeatureSubScreen::sub_screens_in_bounds(view, &[rect(100., 50., 300., 150.)]),
        [view]
    );
    // A zero-width fold divides only while half opened; a cutout with area
    // always does.
    let fold = |state| DisplayFeature {
        bounds: rect(200., 0., 200., 200.),
        kind: DisplayFeatureType::Fold,
        state,
    };
    assert!(
        DisplayFeatureSubScreen::avoid_bounds(&[fold(DisplayFeatureState::PostureFlat)]).is_empty()
    );
    assert_eq!(
        DisplayFeatureSubScreen::avoid_bounds(&[fold(DisplayFeatureState::PostureHalfOpened)]),
        [rect(200., 0., 200., 200.)]
    );
}

const UPSTREAM_VIEW: Size = Size {
    width: 800.,
    height: 600.,
};

fn feature(bounds: Rect, kind: DisplayFeatureType, state: DisplayFeatureState) -> DisplayFeature {
    DisplayFeature {
        bounds,
        kind,
        state,
    }
}

fn scoped_case(
    features: Vec<DisplayFeature>,
    anchor: Option<Offset>,
    direction: Option<TextDirection>,
    zero: bool,
) -> Result<Rect, incular_widgets::internal::TreeError> {
    use incular_widgets::{Center, MediaQuery, SizedBox, internal::Key};

    let key = Key::from("childKey");
    let leaf = Widget::from(SizedBox::expand()).with_key(key.clone());
    let screen = DisplayFeatureSubScreen::new(leaf);
    let mut child = Widget::from(match anchor {
        Some(anchor) => screen.anchor_point(anchor),
        None => screen,
    });
    if let Some(direction) = direction {
        child = Directionality::new(direction, child).into();
    }
    if zero {
        child = Center::new(SizedBox::shrink().child(child)).into();
    }
    let mut tree = WidgetTree::new();
    tree.mount(
        MediaQuery::new(
            RuntimeEnvironment {
                viewport: UPSTREAM_VIEW,
                display_features: features,
                ..RuntimeEnvironment::default()
            },
            child,
        )
        .into(),
    )?;
    tree.layout(Constraints::tight(UPSTREAM_VIEW))?;
    Ok(tree
        .element_bounds(tree.element_with_key(&key).unwrap())
        .unwrap())
}

fn vertical() -> DisplayFeature {
    feature(
        rect(390., 0., 410., 600.),
        DisplayFeatureType::Hinge,
        DisplayFeatureState::Unknown,
    )
}

fn horizontal() -> DisplayFeature {
    feature(
        rect(0., 290., 800., 310.),
        DisplayFeatureType::Hinge,
        DisplayFeatureState::Unknown,
    )
}

#[test]
fn missing_directionality_is_an_error_without_an_anchor() {
    let error = scoped_case(vec![vertical()], None, None, false).unwrap_err();
    assert!(matches!(
        error,
        incular_widgets::internal::TreeError::MissingDirectionality {
            widget: "DisplayFeatureSubScreen",
            ..
        }
    ));
}

#[test]
fn vertical_horizontal_and_crossed_hinges_select_exact_pinned_rectangles() {
    for (features, anchor, expected) in [
        (
            vec![vertical()],
            Offset::new(600., 300.),
            rect(410., 0., 800., 600.),
        ),
        (
            vec![vertical()],
            Offset::new(f32::INFINITY, f32::INFINITY),
            rect(410., 0., 800., 600.),
        ),
        (
            vec![horizontal()],
            Offset::new(1000., 1000.),
            rect(0., 310., 800., 600.),
        ),
        (
            vec![horizontal(), vertical()],
            Offset::new(1000., 1000.),
            rect(410., 310., 800., 600.),
        ),
    ] {
        assert_eq!(
            scoped_case(features, Some(anchor), None, false).unwrap(),
            expected
        );
    }
}

#[test]
fn nonspanning_notches_and_flat_fold_never_crop_the_screen() {
    let features = vec![
        feature(
            rect(100., 0., 700., 100.),
            DisplayFeatureType::Cutout,
            DisplayFeatureState::Unknown,
        ),
        feature(
            rect(100., 500., 700., 600.),
            DisplayFeatureType::Cutout,
            DisplayFeatureState::Unknown,
        ),
        feature(
            rect(0., 300., 800., 300.),
            DisplayFeatureType::Fold,
            DisplayFeatureState::PostureFlat,
        ),
    ];
    assert_eq!(
        scoped_case(features, None, Some(TextDirection::Ltr), false).unwrap(),
        rect(0., 0., 800., 600.)
    );
    let fold = feature(
        rect(0., 300., 800., 300.),
        DisplayFeatureType::Fold,
        DisplayFeatureState::PostureHalfOpened,
    );
    assert_eq!(
        scoped_case(vec![fold], Some(Offset::new(1000., 1000.)), None, false).unwrap(),
        rect(0., 300., 800., 600.)
    );
}

#[test]
fn display_feature_sub_screen_at_zero_size_is_finite() {
    assert_eq!(
        scoped_case(vec![], None, Some(TextDirection::Ltr), true)
            .unwrap()
            .size,
        Size::ZERO
    );
}

#[test]
fn media_query_sub_screen_adjusts_all_three_inset_families_and_retains_other_data() {
    use incular_widgets::{MediaQuery, MediaQueryData, SizedBox};

    let cutout = feature(
        rect(70., 10., 74., 14.),
        DisplayFeatureType::Cutout,
        DisplayFeatureState::Unknown,
    );
    let base = RuntimeEnvironment {
        viewport: Size::new(82., 40.),
        scale_factor: 2.,
        text_scale: 1.2,
        safe_insets: EdgeInsets::only(3., 1., 2., 4.),
        view_padding: EdgeInsets::only(10., 6., 8., 12.),
        view_insets: EdgeInsets::only(7., 5., 6., 8.),
        display_features: vec![
            feature(
                rect(40., 0., 42., 40.),
                DisplayFeatureType::Hinge,
                DisplayFeatureState::PostureFlat,
            ),
            cutout,
        ],
        always_use_24_hour_format: true,
        accessible_navigation: true,
        invert_colors: true,
        reduced_motion: true,
        bold_text: true,
        high_contrast: true,
        on_off_switch_labels: true,
        supports_announce: true,
        ..Default::default()
    };
    let remaining_safe = EdgeInsets::only(0., 1., 2., 4.);
    let remaining_view_padding = EdgeInsets::only(4., 6., 8., 12.);
    let remaining_view_insets = EdgeInsets::only(0., 5., 6., 8.);
    for (input, screen, expected) in [
        (
            base.clone(),
            rect(20., 10., 40., 20.),
            MediaQueryData {
                safe_insets: EdgeInsets::ZERO,
                view_padding: EdgeInsets::ZERO,
                view_insets: EdgeInsets::ZERO,
                display_features: vec![],
                ..base.clone()
            },
        ),
        (
            MediaQueryData {
                view_padding: EdgeInsets::only(46., 6., 8., 12.),
                ..base.clone()
            },
            rect(42., 0., 82., 40.),
            MediaQueryData {
                safe_insets: remaining_safe,
                view_padding: remaining_view_padding,
                view_insets: remaining_view_insets,
                display_features: vec![cutout],
                ..base.clone()
            },
        ),
    ] {
        let seen = Rc::new(RefCell::new(None));
        let child: Widget = Builder::new({
            let seen = seen.clone();
            move |context| {
                *seen.borrow_mut() = context.depend_on::<MediaQueryData>();
                SizedBox::shrink().into()
            }
        })
        .into();
        let scope = Builder::new(move |context| {
            MediaQuery::new(
                context
                    .depend_on::<MediaQueryData>()
                    .unwrap()
                    .remove_display_features(screen),
                child.clone(),
            )
            .into()
        });
        let mut tree = WidgetTree::new();
        tree.mount(MediaQuery::new(input, scope).into()).unwrap();
        tree.layout(Constraints::tight(UPSTREAM_VIEW)).unwrap();
        assert_eq!(*seen.borrow(), Some(expected));
    }
}
