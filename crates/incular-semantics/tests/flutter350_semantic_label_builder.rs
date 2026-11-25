use incular_semantics::{
    SemanticsLabelBuilder,
    TextDirection::{Ltr, Rtl},
};

fn append(builder: &mut SemanticsLabelBuilder, labels: &[&str]) {
    for label in labels {
        builder.add_part(label);
    }
}

#[test]
fn default_separator_and_live_count() {
    let mut b = SemanticsLabelBuilder::new();
    assert!(b.is_empty());
    assert_eq!(b.len(), 0);
    b.add_part("Hello");
    assert!(!b.is_empty());
    assert_eq!(b.len(), 1);
    b.add_part("world");
    assert_eq!(b.len(), 2);
    assert_eq!(b.build(), "Hello world");
}

#[test]
fn custom_separator() {
    let mut b = SemanticsLabelBuilder::with_options(", ", None);
    append(&mut b, &["One", "Two", "Three"]);
    assert_eq!(b.build(), "One, Two, Three");
}

#[test]
fn empty_separator() {
    let mut b = SemanticsLabelBuilder::with_options("", None);
    append(&mut b, &["Hello", "World"]);
    assert_eq!(b.build(), "HelloWorld");
}

#[test]
fn ignores_empty_parts() {
    let mut b = SemanticsLabelBuilder::new();
    append(&mut b, &["Hello", "", "world"]);
    assert_eq!(b.build(), "Hello world");
    assert_eq!(b.len(), 2);
}

#[test]
fn single_part() {
    let mut b = SemanticsLabelBuilder::new();
    b.add_part("Single");
    assert_eq!(b.build(), "Single");
}

#[test]
fn empty_builder() {
    assert_eq!(SemanticsLabelBuilder::new().build(), "");
}

#[test]
fn clear_functionality() {
    let mut b = SemanticsLabelBuilder::new();
    append(&mut b, &["Hello", "world"]);
    assert_eq!(b.len(), 2);
    b.clear();
    assert!(b.is_empty());
    assert_eq!(b.len(), 0);
    assert_eq!(b.build(), "");
}

#[test]
fn reusable_builder() {
    let mut b = SemanticsLabelBuilder::new();
    append(&mut b, &["First", "use"]);
    assert_eq!(b.build(), "First use");
    b.clear();
    append(&mut b, &["Second", "use"]);
    assert_eq!(b.build(), "Second use");
}

#[test]
fn reuse_without_clearing() {
    let mut b = SemanticsLabelBuilder::new();
    append(&mut b, &["First", "batch"]);
    assert_eq!(b.build(), "First batch");
    assert_eq!(b.len(), 2);
    append(&mut b, &["Second", "batch"]);
    assert_eq!(b.build(), "First batch Second batch");
    assert_eq!(b.len(), 4);
    b.add_part("Final");
    assert_eq!(b.build(), "First batch Second batch Final");
    assert_eq!(b.len(), 5);
}

#[test]
fn no_embedding_when_the_overall_direction_is_unspecified() {
    let mut b = SemanticsLabelBuilder::new();
    b.add_part_with_direction("Hello", Some(Ltr));
    b.add_part_with_direction("مرحبا", Some(Rtl));
    assert_eq!(b.build(), "Hello مرحبا");
}

#[test]
fn ltr_label_embeds_rtl_parts_exactly() {
    let mut b = SemanticsLabelBuilder::with_options(" ", Some(Ltr));
    b.add_part_with_direction("Hello", Some(Ltr));
    b.add_part_with_direction("مرحبا", Some(Rtl));
    b.add_part_with_direction("world", Some(Ltr));
    assert_eq!(b.build(), "Hello \u{202b}مرحبا\u{202c} world");
}

#[test]
fn rtl_label_embeds_ltr_parts_exactly() {
    let mut b = SemanticsLabelBuilder::with_options(" ", Some(Rtl));
    b.add_part_with_direction("مرحبا", Some(Rtl));
    b.add_part_with_direction("Hello", Some(Ltr));
    assert_eq!(b.build(), "مرحبا \u{202a}Hello\u{202c}");
}

#[test]
fn matching_part_directions_emit_no_embedding() {
    let mut b = SemanticsLabelBuilder::with_options(" ", Some(Ltr));
    b.add_part_with_direction("Hello", Some(Ltr));
    b.add_part_with_direction("world", Some(Ltr));
    assert_eq!(b.build(), "Hello world");
}

#[test]
fn unspecified_part_direction_uses_overall_direction() {
    let mut b = SemanticsLabelBuilder::with_options(" ", Some(Ltr));
    append(&mut b, &["Hello", "world"]);
    assert_eq!(b.build(), "Hello world");
}

#[test]
fn multiple_mixed_direction_parts_preserve_separators() {
    let mut b = SemanticsLabelBuilder::with_options(", ", Some(Ltr));
    b.add_part_with_direction("Welcome", Some(Ltr));
    b.add_part_with_direction("مرحبا", Some(Rtl));
    b.add_part_with_direction("שלום", Some(Rtl));
    b.add_part_with_direction("to our app", Some(Ltr));
    assert_eq!(
        b.build(),
        "Welcome, \u{202b}مرحبا\u{202c}, \u{202b}שלום\u{202c}, to our app"
    );
}

#[test]
fn very_long_labels() {
    let mut b = SemanticsLabelBuilder::new();
    b.add_part(&"A".repeat(1000));
    b.add_part("short");
    let label = b.build();
    assert_eq!(label.len(), 1006);
    assert!(label.starts_with("AAAA") && label.ends_with(" short"));
}

#[test]
fn many_parts() {
    let mut b = SemanticsLabelBuilder::new();
    for i in 0..100 {
        b.add_part(&format!("part{i}"));
    }
    let label = b.build();
    assert!(label.starts_with("part0 part1") && label.ends_with("part98 part99"));
    assert_eq!(b.len(), 100);
}

#[test]
fn special_separator_characters() {
    let mut b = SemanticsLabelBuilder::with_options(" | ", None);
    append(&mut b, &["first", "second", "third"]);
    assert_eq!(b.build(), "first | second | third");
}

#[test]
fn unicode_characters_in_content() {
    let mut b = SemanticsLabelBuilder::new();
    append(&mut b, &["Emoji: 😀🎉", "Math: ∑∆π", "Currency: €£¥"]);
    assert_eq!(b.build(), "Emoji: 😀🎉 Math: ∑∆π Currency: €£¥");
}

#[test]
fn first_part_always_stays_unwrapped_even_when_it_disagrees() {
    let mut b = SemanticsLabelBuilder::with_options(" ", Some(Ltr));
    b.add_part_with_direction("مرحبا", Some(Rtl));
    assert_eq!(b.build(), "مرحبا");
    b.add_part_with_direction("שלום", Some(Rtl));
    assert_eq!(b.build(), "مرحبا \u{202b}שלום\u{202c}");
}
