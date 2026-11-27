use incular_layout::BaselineOffset as Baseline;

#[test]
fn a_missing_baseline_is_the_identity_of_minimum() {
    let missing = Baseline::NO_BASELINE;
    let one = Baseline::new(Some(1.));
    let two = Baseline::new(Some(2.));
    assert_eq!(missing.min_of(missing), missing);
    assert_eq!(missing.min_of(one), one);
    assert_eq!(one.min_of(missing), one);
    assert_eq!(two.min_of(one), one);
    assert_eq!(one.min_of(two), one);
}

#[test]
fn translating_an_offset_does_not_invent_a_missing_baseline() {
    assert_eq!(Baseline::NO_BASELINE + 2., Baseline::NO_BASELINE);
    assert_eq!(Baseline::new(Some(1.)) + 2., Baseline::new(Some(3.)));
    assert_eq!((Baseline::new(Some(1.)) + -3.).offset(), Some(-2.));
}
