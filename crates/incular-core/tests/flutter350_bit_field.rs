use incular_core::{BitField, K_MAX_UNSIGNED_SMI};

#[derive(Clone, Copy)]
#[repr(usize)]
enum Flag {
    C = 2,
    D,
    E,
}

impl From<Flag> for usize {
    fn from(flag: Flag) -> Self {
        flag as Self
    }
}

#[test]
fn set_clear_and_reset_leave_neighbor_flags_untouched() {
    let mut field = BitField::<Flag>::new(8);
    assert!(!field[Flag::D]);

    field.set(Flag::D, true);
    field.set(Flag::E, true);
    assert!(!field[Flag::C]);
    assert!(field[Flag::D]);
    assert!(field[Flag::E]);

    field.set(Flag::E, false);
    assert!(!field[Flag::C]);
    assert!(field[Flag::D]);
    assert!(!field[Flag::E]);

    for fill in [false, true] {
        field.reset(fill);
        for flag in [Flag::C, Flag::D, Flag::E] {
            assert_eq!(field[flag], fill);
        }
    }
}

#[test]
fn filled_constructor_supports_both_defaults() {
    for fill in [true, false] {
        assert_eq!(BitField::<Flag>::filled(8, fill).get(Flag::D), fill);
    }
}

#[test]
fn boundary_positions_can_be_mutated_without_interfering() {
    assert_eq!(K_MAX_UNSIGNED_SMI, 0x3fff_ffff_ffff_ffff);
    let mut field = BitField::<usize>::filled(62, true);
    assert_eq!(field.len(), 62);
    assert!(field[0]);
    assert!(field[61]);
    field.set(61, false);
    assert!(field[0]);
    assert!(field[60]);
    assert!(!field[61]);
    field.reset(false);
    field.set(61, true);
    assert!(!field[0]);
    assert!(!field[60]);
    assert!(field[61]);
    assert!(BitField::<usize>::new(0).is_empty());
}

#[test]
#[should_panic(expected = "bit field length must be at most 62")]
fn capacity_above_62_is_rejected() {
    let _ = BitField::<usize>::new(63);
}

#[test]
#[should_panic(expected = "bit field index 8 outside length 8")]
fn index_equal_to_the_length_is_not_accessible() {
    let mut field = BitField::<usize>::new(8);
    field.set(8, true);
}
