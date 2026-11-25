use incular_animation::Priority;

#[test]
fn flutter_priority_operators_clamp_each_offset() {
    let max = Priority::MAX_OFFSET;
    let higher = Priority::IDLE + (max + 100);
    assert_eq!(higher.value(), Priority::IDLE.value() + max);

    let lower = Priority::ANIMATION - (max + 100);
    assert_eq!(lower.value(), Priority::ANIMATION.value() - max);
}

#[test]
fn relative_priorities_accumulate_and_do_not_overflow_at_edge_offsets() {
    assert_eq!(Priority::default(), Priority::IDLE);
    assert_eq!(Priority::ANIMATION.value(), 100_000);
    assert_eq!(Priority::TOUCH.value(), 200_000);
    assert!(Priority::TOUCH > Priority::ANIMATION);
    let max = Priority::MAX_OFFSET;
    assert_eq!((Priority::IDLE + max + max).value(), 2 * max);
    assert_eq!((Priority::IDLE + i64::MIN).value(), -max);
    assert_eq!((Priority::IDLE - i64::MIN).value(), max);
    assert_eq!((Priority::IDLE + i64::MAX).value(), max);
    assert_eq!((Priority::IDLE - i64::MAX).value(), -max);
}
