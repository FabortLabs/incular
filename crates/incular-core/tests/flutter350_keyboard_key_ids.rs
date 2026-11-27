use incular_core::{LogicalKeyboardKey as Logical, PhysicalKeyboardKey as Physical};

#[test]
fn physical_lookup_distinguishes_a_letter_control_and_modifier() {
    for (code, key) in [
        (0x0007_0004, Physical::KEY_A),
        (0x0007_0029, Physical::ESCAPE),
        (0x0007_00e1, Physical::SHIFT_LEFT),
    ] {
        assert_eq!(Physical::find_key_by_code(code), Some(key));
        assert_eq!(key.usb_hid_usage(), code);
    }
    assert_eq!(Physical::find_key_by_code(0x0001_2345), None);
}

#[test]
fn equal_physical_usage_codes_denote_the_same_key() {
    assert_eq!(
        Physical::KEY_A,
        Physical::new(Physical::KEY_A.usb_hid_usage())
    );
    let value = std::hint::black_box(0x0001_2345);
    assert_eq!(Physical::new(0x0001_2345), Physical::new(value));
    assert_ne!(Physical::new(value), Physical::KEY_A);
}

#[test]
fn physical_debug_names_use_catalog_entries_and_padded_unknown_codes() {
    for (key, expected) in [
        (Physical::KEY_A, "Key A"),
        (Physical::BACKSLASH, "Backslash"),
        (Physical::new(0x0001_2345), "Key with ID 0x00012345"),
    ] {
        assert_eq!(
            key.debug_name().as_deref(),
            cfg!(debug_assertions).then_some(expected)
        );
    }
}

#[test]
fn logical_lookup_distinguishes_a_letter_control_and_modifier() {
    for key in [Logical::KEY_A, Logical::ESCAPE, Logical::SHIFT_LEFT] {
        assert_eq!(Logical::find_key_by_key_id(key.key_id()), Some(key));
    }
    assert_eq!(Logical::find_key_by_key_id(0x1_0001_2345), None);
}

#[test]
fn logical_controls_have_their_char_codes_in_the_unprintable_plane() {
    for (key, control) in [
        (Logical::BACKSPACE, 0x08),
        (Logical::TAB, 0x09),
        (Logical::ENTER, 0x0d),
        (Logical::ESCAPE, 0x1b),
        (Logical::DELETE, 0x7f),
    ] {
        assert_eq!(key.key_id(), Logical::UNPRINTABLE_PLANE + control);
    }
}

const SIDED: [[Logical; 2]; 4] = [
    [Logical::SHIFT_LEFT, Logical::SHIFT_RIGHT],
    [Logical::CONTROL_LEFT, Logical::CONTROL_RIGHT],
    [Logical::ALT_LEFT, Logical::ALT_RIGHT],
    [Logical::META_LEFT, Logical::META_RIGHT],
];
const ABSTRACT: [Logical; 4] = [
    Logical::SHIFT,
    Logical::CONTROL,
    Logical::ALT,
    Logical::META,
];

#[test]
fn both_modifier_sides_report_the_same_abstract_synonym() {
    for ([left, right], synonym) in SIDED.into_iter().zip(ABSTRACT) {
        assert_eq!(left.synonyms(), &[synonym]);
        assert_eq!(right.synonyms(), &[synonym]);
    }
    assert!(Logical::KEY_A.synonyms().is_empty());
    assert!(Logical::SHIFT.synonyms().is_empty());
}

#[test]
fn collapsing_modifier_synonyms_deduplicates_both_sides() {
    assert!(Logical::collapse_synonyms([]).is_empty());
    for keys in [
        SIDED.into_iter().map(|pair| pair[0]).collect::<Vec<_>>(),
        SIDED.into_iter().map(|pair| pair[1]).collect::<Vec<_>>(),
        SIDED.into_iter().flatten().collect::<Vec<_>>(),
    ] {
        assert_eq!(
            Logical::collapse_synonyms(keys)
                .into_iter()
                .collect::<Vec<_>>(),
            ABSTRACT
        );
    }
    let expanded = Logical::expand_synonyms(ABSTRACT.into_iter().chain([Logical::KEY_A]));
    assert_eq!(
        expanded.into_iter().collect::<Vec<_>>(),
        SIDED
            .into_iter()
            .flatten()
            .chain([Logical::KEY_A])
            .collect::<Vec<_>>()
    );
}

#[test]
fn equal_logical_ids_denote_the_same_key() {
    assert_eq!(Logical::KEY_A, Logical::new(Logical::KEY_A.key_id()));
    let value = std::hint::black_box(0x0001_2345);
    assert_eq!(Physical::new(0x0001_2345), Physical::new(value));
    assert_ne!(Logical::KEY_A, Logical::new(0x41));
}

#[test]
fn logical_labels_separate_printable_unicode_and_named_controls() {
    for (key, label) in [
        (Logical::KEY_A, "A"),
        (Logical::BACKSLASH, "\\"),
        (Logical::new(0xd9), "Ù"),
        (Logical::new(0xf9), "Ù"),
        (Logical::SHIFT_LEFT, "Shift Left"),
        (Logical::NUMPAD_DECIMAL, "Numpad Decimal"),
        (Logical::NUMPAD1, "Numpad 1"),
        (Logical::DELETE, "Delete"),
        (Logical::F12, "F12"),
        (Logical::MEDIA_PLAY, "Media Play"),
        (Logical::new(0x1_0001_2345), ""),
    ] {
        assert_eq!(key.key_label(), label);
    }
}

#[test]
fn logical_debug_names_prefer_descriptions_and_format_unknown_ids() {
    for (key, expected) in [
        (Logical::KEY_A, "Key A"),
        (Logical::BACKSLASH, "Backslash"),
        (Logical::new(0xd9), "Key Ù"),
        (Logical::MEDIA_PLAY, "Media Play"),
        (Logical::new(0x1_0001_2345), "Key with ID 0x00100012345"),
    ] {
        assert_eq!(
            key.debug_name().as_deref(),
            cfg!(debug_assertions).then_some(expected)
        );
    }
}
