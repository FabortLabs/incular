use incular_mobile::text_offsets::{utf8_offset, utf16_offset};

#[test]
fn offsets_convert_between_utf16_and_utf8() {
    let text = "a🙂é";
    // 'a' is one unit and one byte; the emoji two units and four bytes; 'é'
    // one unit and two bytes.
    assert_eq!(
        [0, 1, 3, 4].map(|units| utf8_offset(text, units)),
        [0, 1, 5, 7]
    );
    assert_eq!(
        [0, 1, 5, 7].map(|bytes| utf16_offset(text, bytes)),
        [0, 1, 3, 4]
    );
    // Inside a surrogate pair or past the end, the next boundary.
    assert_eq!(utf8_offset(text, 2), 5);
    assert_eq!(utf8_offset(text, 99), text.len());
}
