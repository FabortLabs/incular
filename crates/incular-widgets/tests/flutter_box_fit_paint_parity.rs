//! Positive input sizes from Flutter 3.50.0's `painting/box_fit_test.dart`
//! "applyBoxFit" registration, exercised through public retained Image paint.

use incular_config::Constraints;
use incular_core::{Offset, Rect, Size};
use incular_image::ImageHandle;
use incular_rendering::PaintCommand;
use incular_widgets::{
    BoxFit, Directionality, FittedSizes, Image, TextDirection, apply_box_fit, internal::WidgetTree,
};

#[derive(Clone, Copy, Debug)]
struct Case {
    fit: BoxFit,
    input: (u32, u32),
    output: (f32, f32),
    source_rect: [f32; 4],
    destination_rect: [f32; 4],
    crops: bool,
}

// The upstream results supply the sizes; the origins center each resulting
// rectangle in the input image or output box, as Flutter paintImage does.
const CASES: [Case; 6] = [
    Case {
        fit: BoxFit::ScaleDown,
        input: (100, 1000),
        output: (200., 2000.),
        source_rect: [0., 0., 100., 1000.],
        destination_rect: [50., 500., 100., 1000.],
        crops: false,
    },
    Case {
        fit: BoxFit::ScaleDown,
        input: (300, 3000),
        output: (200., 2000.),
        source_rect: [0., 0., 300., 3000.],
        destination_rect: [0., 0., 200., 2000.],
        crops: false,
    },
    Case {
        fit: BoxFit::FitWidth,
        input: (2000, 400),
        output: (1000., 100.),
        source_rect: [0., 100., 2000., 200.],
        destination_rect: [0., 0., 1000., 100.],
        crops: true,
    },
    Case {
        fit: BoxFit::FitWidth,
        input: (2000, 400),
        output: (1000., 300.),
        source_rect: [0., 0., 2000., 400.],
        destination_rect: [0., 50., 1000., 200.],
        crops: false,
    },
    Case {
        fit: BoxFit::FitHeight,
        input: (400, 2000),
        output: (100., 1000.),
        source_rect: [100., 0., 200., 2000.],
        destination_rect: [0., 0., 100., 1000.],
        crops: true,
    },
    Case {
        fit: BoxFit::FitHeight,
        input: (400, 2000),
        output: (300., 1000.),
        source_rect: [0., 0., 400., 2000.],
        destination_rect: [50., 0., 200., 1000.],
        crops: false,
    },
];

fn rect([x, y, width, height]: [f32; 4]) -> Rect {
    Rect::from_origin_size(Offset::new(x, y), Size::new(width, height))
}

fn painted_image_rects(case: Case, mirror_rtl: bool) -> (Rect, Rect) {
    let (width, height) = case.input;
    let decoded = ImageHandle::from_rgba8(width, height, vec![255; (width * height * 4) as usize])
        .expect("decoded image");
    let image = Image::new(decoded)
        .width(case.output.0)
        .height(case.output.1)
        .fit(case.fit)
        .match_text_direction(mirror_rtl);
    let mut tree = WidgetTree::new();
    tree.mount(Directionality::new(TextDirection::Rtl, image).into())
        .expect("mount image");
    tree.layout(Constraints::tight(Size::new(case.output.0, case.output.1)))
        .expect("layout image");
    tree.update_compositor(incular_core::time::Instant::now())
        .expect("composite image");
    let list = tree.paint();
    let mut images = list.commands().iter().filter_map(|command| match command {
        PaintCommand::Image {
            source,
            destination,
            ..
        } => Some((*source, *destination)),
        _ => None,
    });
    let drawn = images.next().expect("positive fitted image draws");
    assert!(images.next().is_none(), "exactly one nonrepeated image");
    drawn
}

fn check_cases(cases: impl IntoIterator<Item = Case>, mirror_rtl: bool) {
    let mut mismatches = Vec::new();
    for case in cases {
        let actual = painted_image_rects(case, mirror_rtl);
        let expected = (rect(case.source_rect), rect(case.destination_rect));
        if actual != expected {
            mismatches.push((case, actual, expected));
        }
    }
    assert!(
        mismatches.is_empty(),
        "image fit mismatches (case, actual paint rects, Flutter paint rects): {mismatches:#?}"
    );
}

#[test]
fn plain_image_fits_non_cropping_branches_of_scale_down_fit_width_and_fit_height() {
    check_cases(CASES.into_iter().filter(|case| !case.crops), false);
}

#[test]
fn plain_image_crops_for_fit_width_and_fit_height_when_the_other_axis_would_overflow() {
    check_cases(CASES.into_iter().filter(|case| case.crops), false);
}

#[test]
fn rtl_mirrored_image_uses_flutter_source_and_destination_sizes_for_all_six_inputs() {
    // RTL with matchTextDirection selects the separate paintImage path
    // without changing the centered image's source or destination rect.
    check_cases(CASES, true);
}

#[test]
fn apply_box_fit_matches_every_pinned_source_and_destination_size() {
    for case in CASES {
        assert_eq!(
            apply_box_fit(
                case.fit,
                Size::new(case.input.0 as f32, case.input.1 as f32),
                Size::new(case.output.0, case.output.1),
            ),
            FittedSizes::new(
                rect(case.source_rect).size,
                rect(case.destination_rect).size
            ),
            "{case:?}"
        );
    }

    for fit in [
        BoxFit::Fill,
        BoxFit::Contain,
        BoxFit::Cover,
        BoxFit::FitWidth,
        BoxFit::FitHeight,
        BoxFit::None,
        BoxFit::ScaleDown,
    ] {
        for invalid in [-1., 0.] {
            for (input, output) in [
                (
                    Size {
                        width: 400. * invalid,
                        height: 2000.,
                    },
                    Size::new(100., 1000.),
                ),
                (
                    Size {
                        width: 400.,
                        height: 2000. * invalid,
                    },
                    Size::new(100., 1000.),
                ),
                (
                    Size::new(400., 2000.),
                    Size {
                        width: 100. * invalid,
                        height: 1000.,
                    },
                ),
                (
                    Size::new(400., 2000.),
                    Size {
                        width: 100.,
                        height: 1000. * invalid,
                    },
                ),
            ] {
                assert_eq!(
                    apply_box_fit(fit, input, output),
                    FittedSizes::default(),
                    "{fit:?}, {input:?}, {output:?}"
                );
            }
        }
    }
}
