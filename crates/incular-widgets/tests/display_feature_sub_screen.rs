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
    // A centered square leaves a band on every side.
    assert_eq!(
        DisplayFeatureSubScreen::sub_screens_in_bounds(view, &[rect(100., 50., 300., 150.)]),
        [
            rect(0., 0., 100., 200.),
            rect(0., 0., 400., 50.),
            rect(300., 0., 400., 200.),
            rect(0., 150., 400., 200.),
        ]
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
