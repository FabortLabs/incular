//! NavigationToolbar geometry against the pinned neutral Flutter widget.
use incular_config::{Alignment, Constraints};
use incular_core::{Color, Size};
use incular_widgets::{
    Align, Directionality, NavigationToolbar, SizedBox, TextDirection, Widget,
    internal::{Key, WidgetTree},
};

fn measure(
    [a, b, c]: [f32; 3],
    centered: bool,
    spacing: Option<f32>,
    direction: TextDirection,
) -> [[f32; 4]; 3] {
    let chip = |i, w, h| Widget::box_(Size::new(w, h), Color::WHITE).with_key(Key::Value(i));
    let (leading, middle, trailing) = (chip(0, a, 24.), chip(1, b, 20.), chip(2, c, 24.));
    let toolbar = match spacing {
        Some(s) => NavigationToolbar::builder()
            .leading(leading)
            .middle(middle)
            .trailing(trailing)
            .center_middle(centered)
            .middle_spacing(s)
            .build(),
        None => NavigationToolbar::new()
            .leading(leading)
            .middle(middle)
            .trailing(trailing)
            .center_middle(centered),
    };
    let w = Directionality::new(
        direction,
        Align::new(
            Alignment::TOP_LEFT,
            SizedBox::new().width(300.).height(60.).child(toolbar),
        ),
    );
    let mut t = WidgetTree::new();
    t.mount(w.into()).unwrap();
    t.layout(Constraints::tight(Size::new(800., 600.))).unwrap();
    std::array::from_fn(|i| {
        let q = t
            .element_bounds(t.element_with_key(&Key::Value(i as u64)).unwrap())
            .unwrap();
        [q.origin.x, q.origin.y, q.size.width, q.size.height]
    })
}
const LTR: TextDirection = TextDirection::Ltr;
const ASYMMETRIC: [f32; 3] = [80., 40., 20.];

#[test]
fn asymmetric_centered_slots_use_the_whole_bar_and_stretch_leading() {
    assert_eq!(
        measure(ASYMMETRIC, true, None, LTR),
        [
            [0., 0., 80., 60.],
            [130., 20., 40., 20.],
            [280., 18., 20., 24.]
        ]
    );
}
#[test]
fn equal_side_widths_control() {
    assert_eq!(
        measure([50., 40., 50.], true, None, LTR)[1],
        [130., 20., 40., 20.]
    );
}
#[test]
fn default_and_explicit_uncentered_gap_from_leading() {
    let actual = [None, Some(24.)].map(|s| measure(ASYMMETRIC, false, s, LTR)[1]);
    assert_eq!(actual, [[96., 20., 40., 20.], [104., 20., 40., 20.]]);
}
#[test]
fn uncentered_rtl_mirrors_every_slot() {
    assert_eq!(
        measure(ASYMMETRIC, false, None, TextDirection::Rtl),
        [
            [220., 0., 80., 60.],
            [164., 20., 40., 20.],
            [0., 18., 20., 24.]
        ]
    );
}
#[test]
fn middle_fits_between_leading_and_trailing_reservations() {
    let actual = [[200., 100., 20.], [20., 100., 140.], [180., 100., 100.]]
        .map(|sizes| measure(sizes, true, None, LTR)[1]);
    assert_eq!(
        actual,
        [
            [216., 20., 48., 20.],
            [44., 20., 100., 20.],
            [196., 20., 0., 20.]
        ]
    );
}
