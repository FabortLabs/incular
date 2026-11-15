//! paint-020: Flutter's `EdgeInsets.lerp` and `Alignment.lerp` do not clamp
//! `t`, so an overshooting curve extrapolates past both endpoints.

use incular_config::{Alignment, EdgeInsets};
use incular_core::Lerp;

#[test]
fn edge_insets_extrapolate_past_their_endpoints() {
    let begin = EdgeInsets::only(10.0, 10.0, 10.0, 10.0);
    let end = EdgeInsets::only(20.0, 20.0, 20.0, 20.0);
    assert_eq!(begin.lerp(&end, 1.5).left, 25.0);
    assert_eq!(begin.lerp(&end, 0.0), begin);
    assert_eq!(begin.lerp(&end, 1.0), end);
}

#[test]
fn alignment_extrapolates_past_its_endpoints() {
    let begin = Alignment::new(0.0, 0.0);
    let end = Alignment::new(1.0, 1.0);
    let overshoot = begin.lerp(&end, 1.5);
    assert_eq!((overshoot.x, overshoot.y), (1.5, 1.5));
}
