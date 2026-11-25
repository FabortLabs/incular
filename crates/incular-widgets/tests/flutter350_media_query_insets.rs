use std::{cell::RefCell, rc::Rc};

use incular_config::{
    Constraints, DisplayFeature, DisplayFeatureState, DisplayFeatureType, EdgeInsets,
    NavigationMode, RuntimeEnvironment,
};
use incular_core::{Rect, Size};
use incular_widgets::{
    Builder, MediaQuery, MediaQueryData, MediaQueryEdges, SizedBox, Widget, internal::WidgetTree,
};

#[derive(Clone, Copy, Debug)]
enum Operation {
    Padding,
    Insets,
    ViewPadding,
}

impl Operation {
    fn wrap(self, child: Widget, edges: MediaQueryEdges) -> Widget {
        match self {
            Self::Padding => MediaQuery::remove_padding(child, edges),
            Self::Insets => MediaQuery::remove_view_insets(child, edges),
            Self::ViewPadding => MediaQuery::remove_view_padding(child, edges),
        }
    }
    fn apply(self, data: MediaQueryData, edges: MediaQueryEdges) -> MediaQueryData {
        match self {
            Self::Padding => data.remove_padding(edges),
            Self::Insets => data.remove_view_insets(edges),
            Self::ViewPadding => data.remove_view_padding(edges),
        }
    }
}

fn original_inputs(operation: Operation) -> MediaQueryData {
    let minor = EdgeInsets::only(3., 1., 2., 4.);
    let major = EdgeInsets::only(7., 5., 6., 8.);
    let (safe_insets, view_insets) = if matches!(operation, Operation::Padding) {
        (minor, major)
    } else {
        (major, minor)
    };
    RuntimeEnvironment {
        viewport: Size::new(2., 4.),
        scale_factor: 2.,
        text_scale: 1.2,
        safe_insets,
        view_padding: EdgeInsets::only(10., 6., 8., 12.),
        view_insets,
        always_use_24_hour_format: true,
        accessible_navigation: true,
        invert_colors: true,
        reduced_motion: true,
        bold_text: true,
        high_contrast: true,
        on_off_switch_labels: true,
        supports_announce: true,
        navigation_mode: NavigationMode::Directional,
        display_features: vec![DisplayFeature {
            bounds: Rect::default(),
            kind: DisplayFeatureType::Cutout,
            state: DisplayFeatureState::Unknown,
        }],
        ..Default::default()
    }
}

fn compare(operation: Operation, edges: MediaQueryEdges, expected: [EdgeInsets; 3]) {
    let original = original_inputs(operation);
    let [safe_insets, view_padding, view_insets] = expected;
    let expected = MediaQueryData {
        safe_insets,
        view_padding,
        view_insets,
        ..original.clone()
    };
    assert_eq!(operation.apply(original.clone(), edges), expected);
    let seen = Rc::new(RefCell::new(None));
    let child = Builder::new({
        let seen = seen.clone();
        move |context| {
            let current = context.depend_on::<MediaQueryData>().unwrap();
            assert_eq!(context.media_query_padding(), Some(current.safe_insets));
            assert_eq!(
                context.media_query_view_padding(),
                Some(current.view_padding)
            );
            assert_eq!(context.media_query_view_insets(), Some(current.view_insets));
            *seen.borrow_mut() = Some(current);
            SizedBox::shrink().into()
        }
    });
    let mut tree = WidgetTree::new();
    tree.mount(MediaQuery::new(original, operation.wrap(child.into(), edges)).into())
        .unwrap();
    tree.layout(Constraints::loose(Size::new(100., 100.)))
        .unwrap();
    assert_eq!(*seen.borrow(), Some(expected));
}

#[test]
fn remove_padding_all_or_top_only_matches_flutter() {
    let operation = Operation::Padding;
    let original = original_inputs(operation);
    compare(
        operation,
        MediaQueryEdges::ALL,
        [EdgeInsets::ZERO, original.view_insets, original.view_insets],
    );
    compare(
        operation,
        MediaQueryEdges {
            top: true,
            ..MediaQueryEdges::NONE
        },
        [
            EdgeInsets::only(3., 0., 2., 4.),
            EdgeInsets::only(10., 5., 8., 12.),
            original.view_insets,
        ],
    );
}

#[test]
fn remove_view_insets_all_or_bottom_only_matches_flutter() {
    let operation = Operation::Insets;
    let original = original_inputs(operation);
    compare(
        operation,
        MediaQueryEdges::ALL,
        [original.safe_insets, original.safe_insets, EdgeInsets::ZERO],
    );
    compare(
        operation,
        MediaQueryEdges {
            bottom: true,
            ..MediaQueryEdges::NONE
        },
        [
            original.safe_insets,
            EdgeInsets::only(10., 6., 8., 8.),
            EdgeInsets::only(3., 1., 2., 0.),
        ],
    );
}

#[test]
fn remove_view_padding_all_or_left_only_matches_flutter() {
    let operation = Operation::ViewPadding;
    let original = original_inputs(operation);
    compare(
        operation,
        MediaQueryEdges::ALL,
        [EdgeInsets::ZERO, EdgeInsets::ZERO, original.view_insets],
    );
    compare(
        operation,
        MediaQueryEdges {
            left: true,
            ..MediaQueryEdges::NONE
        },
        [
            EdgeInsets::only(0., 5., 6., 8.),
            EdgeInsets::only(0., 6., 8., 12.),
            original.view_insets,
        ],
    );
}

#[test]
fn zero_mask_is_identity_and_subtraction_does_not_go_negative() {
    for op in [
        Operation::Padding,
        Operation::Insets,
        Operation::ViewPadding,
    ] {
        let input = original_inputs(op);
        assert_eq!(op.apply(input.clone(), MediaQueryEdges::NONE), input);
    }
    let input = MediaQueryData {
        safe_insets: EdgeInsets::all(2.),
        view_padding: EdgeInsets::all(1.),
        view_insets: EdgeInsets::all(3.),
        ..Default::default()
    };
    assert_eq!(
        input
            .clone()
            .remove_view_insets(MediaQueryEdges::ALL)
            .view_padding,
        EdgeInsets::ZERO
    );
    assert_eq!(
        input.remove_padding(MediaQueryEdges::ALL).view_padding,
        EdgeInsets::ZERO
    );
}
