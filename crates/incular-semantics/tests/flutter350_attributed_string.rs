use incular_semantics::{AttributedString, InvalidStringAttribute, StringAttribute};

#[test]
fn concat_matches_flutter_string_attribute_ranges_and_diagnostics() {
    let first =
        AttributedString::try_with_attributes("string1", [StringAttribute::spell_out(0..4)])
            .unwrap();
    let second =
        AttributedString::try_with_attributes("string2", [StringAttribute::locale(0..4, "es-MX")])
            .unwrap();
    let joined = first + second;
    assert_eq!(joined.as_str(), "string1string2");
    assert_eq!(
        joined.attributes(),
        [
            StringAttribute::SpellOut { range: 0..4 },
            StringAttribute::Locale {
                range: 7..11,
                locale: "es-MX".into()
            },
        ]
    );
    assert_eq!(
        joined.to_string(),
        "AttributedString('string1string2', attributes: [SpellOutStringAttribute(TextRange(start: 0, end: 4)), LocaleStringAttribute(TextRange(start: 7, end: 11), es-MX)])",
    );
}

#[test]
fn concat_uses_utf16_not_utf8_and_keeps_supplementary_characters() {
    let first =
        AttributedString::try_with_attributes("😀x", [StringAttribute::spell_out(0..2)]).unwrap();
    let second =
        AttributedString::try_with_attributes("é", [StringAttribute::locale(0..1, "fr")]).unwrap();
    let joined = first + second.clone();
    assert_eq!(joined.as_str(), "😀xé");
    assert_eq!(joined.attributes()[0].range(), &(0..2));
    assert_eq!(joined.attributes()[1], StringAttribute::locale(3..4, "fr"));
    assert_eq!(second.attributes()[0].range(), &(0..1));
}

#[test]
fn empty_operands_do_not_shift_ranges_and_invalid_ranges_are_reported() {
    let attr = StringAttribute::spell_out(0..2);
    let original = AttributedString::try_with_attributes("😀", [attr.clone()]).unwrap();
    assert_eq!(AttributedString::default() + original.clone(), original);
    assert_eq!(original.clone() + AttributedString::default(), original);
    assert_eq!(original.into_parts(), ("😀".into(), vec![attr]));
    assert_eq!(
        AttributedString::try_with_attributes(
            "😀",
            [
                StringAttribute::spell_out(0..1),
                StringAttribute::locale(0..3, "en"),
            ]
        ),
        Err(InvalidStringAttribute {
            attribute_index: 1,
            range: 0..3,
            text_units: 2
        }),
    );
    assert!(AttributedString::try_with_attributes("", [StringAttribute::spell_out(0..0)]).is_err());
}
