use incular_config::{Alignment, EdgeInsets};
use incular_core::{Color, HsvColor, Lerp};
use incular_image::ImageHandle;
use incular_parity_harness::Scenario;
use incular_parity_harness::{FrameHarness, probe};
use incular_widgets::{
    BorderRadius, BoxDecoration, Center, ColoredBox, DecoratedBox, Opacity, RawImage, SizedBox,
    Widget,
};

pub fn scenarios() -> Vec<Scenario> {
    vec![
        ("paint__opacity__phase_values", opacity_phase_values),
        (
            "paint__opacity__zero_keeps_layout",
            opacity_zero_keeps_layout,
        ),
        (
            "paint__decorated_box__solid_background",
            decorated_box_solid_background,
        ),
        (
            "paint__raw_image__intrinsic_scale",
            raw_image_intrinsic_scale,
        ),
        (
            "paint__data__edge_insets_lerp_extrapolated",
            edge_insets_lerp_extrapolated,
        ),
        (
            "paint__data__alignment_lerp_extrapolated",
            alignment_lerp_extrapolated,
        ),
        ("paint__data__hsv_lerp_hue_wrap", hsv_lerp_hue_wrap),
        (
            "paint__data__border_radius_lerp_extrapolated",
            border_radius_lerp_extrapolated,
        ),
    ]
}

const TEAL: Color = Color::rgba(0x00, 0x96, 0x88, 0xFF);
const PURPLE: Color = Color::rgba(0x67, 0x3A, 0xB7, 0xFF);

fn colored(color: Color) -> Widget {
    ColoredBox::new(color, SizedBox::shrink()).into()
}

fn opacity_phase_values(h: &mut FrameHarness) {
    h.mount(|phase| {
        Center::new(Opacity::new(
            if phase == 0 { 0.2 } else { 0.8 },
            probe(
                "box",
                SizedBox::new().width(90.).height(60.).child(colored(TEAL)),
            ),
        ))
        .into()
    });
    h.frames(2);
    h.set_phase(1);
    h.frames(2);
}

fn opacity_zero_keeps_layout(h: &mut FrameHarness) {
    h.mount(|_| {
        Center::new(Opacity::new(
            0.,
            probe(
                "box",
                SizedBox::new()
                    .width(80.)
                    .height(48.)
                    .child(colored(PURPLE)),
            ),
        ))
        .into()
    });
    h.frames(2);
}

fn decorated_box_solid_background(h: &mut FrameHarness) {
    h.mount(|_| {
        Center::new(probe(
            "box",
            DecoratedBox::new(SizedBox::new().width(104.).height(72.))
                .decoration(BoxDecoration::new().color(TEAL)),
        ))
        .into()
    });
    h.frames(2);
}

fn raw_image_intrinsic_scale(h: &mut FrameHarness) {
    let image = ImageHandle::from_rgba8(20, 10, [0x00, 0x96, 0x88, 0xFF].repeat(20 * 10))
        .expect("valid test image");
    h.mount(move |_| {
        Center::new(probe(
            "image",
            RawImage::new().image(image.clone()).scale(2.),
        ))
        .into()
    });
    h.frames(2);
}

fn lerped_edge_insets() -> EdgeInsets {
    EdgeInsets::only(8., 4., 12., 16.).lerp(&EdgeInsets::only(20., 0., 6., 2.), 1.5)
}

fn edge_insets_lerp_extrapolated(h: &mut FrameHarness) {
    h.value("left", || f64::from(lerped_edge_insets().left));
    h.value("top", || f64::from(lerped_edge_insets().top));
    h.value("right", || f64::from(lerped_edge_insets().right));
    h.value("bottom", || f64::from(lerped_edge_insets().bottom));
    h.mount(|_| {
        Center::new(probe(
            "box",
            SizedBox::new().width(40.).height(20.).child(colored(TEAL)),
        ))
        .into()
    });
    h.frames(2);
}

fn lerped_alignment() -> Alignment {
    Alignment::TOP_LEFT.lerp(&Alignment::BOTTOM_RIGHT, 1.5)
}

fn alignment_lerp_extrapolated(h: &mut FrameHarness) {
    h.value("x", || f64::from(lerped_alignment().x));
    h.value("y", || f64::from(lerped_alignment().y));
    h.mount(|_| {
        Center::new(probe(
            "box",
            SizedBox::new().width(40.).height(20.).child(colored(TEAL)),
        ))
        .into()
    });
    h.frames(2);
}

fn hsv_lerp_hue_wrap(h: &mut FrameHarness) {
    h.value("hue", || {
        f64::from(
            HsvColor::new(300., 1., 1., 1.)
                .lerp(&HsvColor::new(360., 1., 1., 1.), 0.5)
                .hue,
        )
    });
    h.value("saturation", || {
        f64::from(
            HsvColor::new(300., 1., 1., 1.)
                .lerp(&HsvColor::new(360., 0.5, 1., 1.), 0.5)
                .saturation,
        )
    });
    h.mount(|_| {
        Center::new(probe(
            "box",
            SizedBox::new().width(40.).height(20.).child(colored(TEAL)),
        ))
        .into()
    });
    h.frames(2);
}

fn border_radius_lerp_extrapolated(h: &mut FrameHarness) {
    h.value("top_left_x", || {
        f64::from(
            BorderRadius::circular(4.)
                .lerp(&BorderRadius::circular(12.), 1.5)
                .top_left
                .x,
        )
    });
    h.mount(|_| {
        Center::new(probe(
            "box",
            SizedBox::new().width(40.).height(20.).child(colored(TEAL)),
        ))
        .into()
    });
    h.frames(2);
}
