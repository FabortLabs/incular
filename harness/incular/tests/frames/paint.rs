use incular_core::Color;
use incular_image::ImageHandle;
use incular_parity_harness::Scenario;
use incular_parity_harness::{FrameHarness, probe};
use incular_widgets::{
    BoxDecoration, Center, ColoredBox, DecoratedBox, Opacity, RawImage, SizedBox, Widget,
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
