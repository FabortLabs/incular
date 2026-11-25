//! Selection and grapheme limits from pinned Flutter services/text_formatter_test.dart.
use incular_text::{TextEditingValue, TextSelection};
use incular_widgets::{
    FilteringTextInputFormatter, LengthLimitingTextInputFormatter, MaxLengthEnforcement,
    TextInputFormatter,
};

fn editing(text: &str, base: usize, extent: usize) -> TextEditingValue {
    TextEditingValue {
        text: text.into(),
        selection: TextSelection::new(base, extent),
        composing: None,
    }
}
fn sample() -> TextEditingValue {
    editing("a1b2c3\nd4e5f6", 3, 9)
}
fn empty() -> TextEditingValue {
    editing("", 0, 0)
}
#[test]
fn single_line_remaps_selection_across_the_removed_newline() {
    let f = FilteringTextInputFormatter::single_line_formatter();
    assert_eq!(
        f.format_edit_update(&empty(), &sample()),
        editing("a1b2c3d4e5f6", 3, 8)
    );
}
#[test]
fn digits_only_remaps_both_selection_endpoints() {
    let f = FilteringTextInputFormatter::digits_only();
    assert_eq!(
        f.format_edit_update(&empty(), &sample()),
        editing("123456", 1, 4)
    );
}
#[test]
fn length_limit_clips_range_and_preserves_valid_selection_start() {
    for (limit, expected) in [(6, editing("a1b2c3", 3, 6)), (2, editing("a1", 2, 2))] {
        let f = LengthLimitingTextInputFormatter::new(limit);
        assert_eq!(f.format_edit_update(&empty(), &sample()), expected);
    }
}
#[test]
fn length_limit_preserves_empty_edit_with_zero_caret() {
    let f = LengthLimitingTextInputFormatter::new(1);
    assert_eq!(f.format_edit_update(&empty(), &empty()), empty());
}
#[test]
fn length_limit_keeps_one_unicorn_or_latin_grapheme() {
    let f = LengthLimitingTextInputFormatter::new(1);
    for (input, first) in [("🦄 ", "🦄"), ("XY", "X")] {
        let caret = first.len();
        assert_eq!(
            f.format_edit_update(&empty(), &editing(input, caret, caret)),
            editing(first, caret, caret)
        );
    }
}
#[test]
fn length_limit_passes_an_edit_below_the_limit() {
    let f = LengthLimitingTextInputFormatter::new(10);
    assert_eq!(
        f.format_edit_update(&editing("aaa", 3, 3), &editing("aaab", 4, 4)),
        editing("aaab", 4, 4)
    );
}
#[test]
fn excess_insert_at_limit_is_rejected_without_losing_existing_text() {
    let f = LengthLimitingTextInputFormatter::new(10);
    let old = editing("aaaaaaaaaa", 5, 5);
    let new = editing("aaaaabbbbbaaaaa", 10, 10);
    assert_eq!(f.format_edit_update(&old, &new), old);
}
#[test]
fn already_over_limit_edit_is_truncated_to_max() {
    let f = LengthLimitingTextInputFormatter::new(10);
    assert_eq!(
        f.format_edit_update(
            &editing("aaaaaaaaaaaaaaaaaaaa", 20, 20),
            &editing("bbbbbbbbbbbbbbbbbbbb", 20, 20)
        ),
        editing("bbbbbbbbbb", 10, 10)
    );
}
#[test]
fn default_enforcement_depends_on_native_platform_and_web_overrides() {
    use MaxLengthEnforcement::{Enforced, TruncateAfterCompositionEnds as Deferred};
    use incular_config::TargetPlatform::{self, *};
    for (platform, native) in [
        (Android, Enforced),
        (Windows, Enforced),
        (MacOs, Deferred),
        (Ios, Deferred),
        (Linux, Deferred),
        (Fuchsia, Deferred),
    ] {
        assert_eq!(
            LengthLimitingTextInputFormatter::default_enforcement(platform, false),
            native
        );
        assert_eq!(
            LengthLimitingTextInputFormatter::default_enforcement(platform, true),
            Deferred
        );
    }
    assert_eq!(
        LengthLimitingTextInputFormatter::new(5).max_length_enforcement,
        LengthLimitingTextInputFormatter::default_enforcement(
            TargetPlatform::host(),
            cfg!(target_arch = "wasm32")
        )
    );
}

#[test]
fn public_truncate_matches_the_three_upstream_ascii_and_unicode_inputs() {
    for (input, expected) in [
        ("01234567890", "0123456789"),
        ("😆01234567890", "😆012345678"),
        ("👨‍👩‍👦01234567890", "👨‍👩‍👦012345678"),
    ] {
        let shortened =
            LengthLimitingTextInputFormatter::truncate(TextEditingValue::new(input), 10);
        assert_eq!(shortened.text, expected);
        assert_eq!(
            shortened.selection,
            TextSelection::collapsed(expected.len())
        );
    }
}

#[test]
fn enforced_deferred_and_unlimited_policies_distinguish_active_and_committed_composition() {
    use MaxLengthEnforcement::{Enforced, None as Unlimited, TruncateAfterCompositionEnds};
    use incular_text::TextRange;
    let enforced = LengthLimitingTextInputFormatter::with_enforcement(2, Enforced);
    let deferred =
        LengthLimitingTextInputFormatter::with_enforcement(2, TruncateAfterCompositionEnds);
    let unlimited = LengthLimitingTextInputFormatter::with_enforcement(2, Unlimited);
    let comp = |text, caret, start, end| TextEditingValue {
        composing: Some(TextRange::new(start, end)),
        ..editing(text, caret, caret)
    };
    let old = editing("a", 1, 1);
    let incoming = comp("abc", 3, 1, 3);
    assert_eq!(
        enforced.format_edit_update(&old, &incoming),
        comp("ab", 2, 1, 2)
    );
    assert_eq!(deferred.format_edit_update(&old, &incoming), incoming);
    assert_eq!(
        deferred.format_edit_update(&incoming, &editing("abc", 3, 3)),
        editing("ab", 2, 2)
    );
    assert_eq!(unlimited.format_edit_update(&old, &incoming), incoming);

    let full = editing("ab", 1, 1);
    let insert = comp("aXb", 2, 1, 2);
    assert_eq!(unlimited.format_edit_update(&full, &insert), insert);
    assert_eq!(
        [
            enforced.format_edit_update(&full, &insert),
            deferred.format_edit_update(&full, &insert)
        ],
        [full.clone(), full]
    );
}

#[test]
fn oversize_replacement_of_selected_text_uses_policy_specific_guard() {
    let enforced =
        LengthLimitingTextInputFormatter::with_enforcement(2, MaxLengthEnforcement::Enforced);
    let deferred = LengthLimitingTextInputFormatter::with_enforcement(
        2,
        MaxLengthEnforcement::TruncateAfterCompositionEnds,
    );
    let selected = editing("ab", 0, 1);
    let oversize = editing("aXYZ", 4, 4);
    assert_eq!(
        enforced.format_edit_update(&selected, &oversize),
        editing("aX", 2, 2)
    );
    assert_eq!(deferred.format_edit_update(&selected, &oversize), selected);

    let exact = editing("🦄b", "🦄".len(), "🦄".len());
    let insert = editing("🦄Xb", "🦄X".len(), "🦄X".len());
    assert_eq!(enforced.format_edit_update(&exact, &insert), exact);
    assert_eq!(deferred.format_edit_update(&exact, &insert), exact);

    let exactly_two = editing("🦄X", "🦄X".len(), "🦄X".len());
    assert_eq!(
        enforced.format_edit_update(&exact, &exactly_two),
        exactly_two
    );
    assert_eq!(
        deferred.format_edit_update(&exact, &exactly_two),
        exactly_two
    );
}
