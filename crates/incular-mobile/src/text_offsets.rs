//! Java strings (Android) and `NSString` (iOS) index text in UTF-16 code
//! units; Incular's text values use UTF-8 byte offsets.

/// The UTF-8 byte offset of UTF-16 offset `utf16` in `text`. An offset inside
/// a surrogate pair, or past the end, moves to the next character boundary.
#[must_use]
pub fn utf8_offset(text: &str, utf16: usize) -> usize {
    let mut units = 0;
    for (index, character) in text.char_indices() {
        if units >= utf16 {
            return index;
        }
        units += character.len_utf16();
    }
    text.len()
}

/// The UTF-16 offset of UTF-8 byte offset `utf8` in `text`. An offset inside
/// a character counts that whole character.
#[must_use]
pub fn utf16_offset(text: &str, utf8: usize) -> usize {
    text.char_indices()
        .take_while(|(index, _)| *index < utf8)
        .map(|(_, character)| character.len_utf16())
        .sum()
}
