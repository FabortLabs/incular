use incular_core::input::is_control_key_label;

#[test]
fn common_control_labels_match_flutter() {
    for label in ["\x08", "\x09", "\x0a", "\x0d", "\x1b", "\x7f"] {
        assert!(is_control_key_label(label), "{label:?}");
    }
    for label in ["A", " ", "~", "\u{a0}"] {
        assert!(!is_control_key_label(label), "{label:?}");
    }
}

#[test]
fn classification_does_not_accept_whole_names_or_non_control_unicode() {
    for label in ["\0", "\x1f", "\u{80}", "\u{9f}"] {
        assert!(is_control_key_label(label), "{label:?}");
    }
    for label in ["", "Enter", "\x08A", "\x08\x09", "🙂", "\u{200d}"] {
        assert!(!is_control_key_label(label), "{label:?}");
    }
}
