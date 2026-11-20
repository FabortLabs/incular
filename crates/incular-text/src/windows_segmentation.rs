//! Use the Windows ICU dictionaries for native UTF-8 word and line boundaries.

use std::{ops::Range, ptr};

use icu_properties::{CodePointMapData, props::LineBreak};
use windows_sys::Win32::Globalization::{
    UBRK_LINE, UBRK_WORD, UBreakIteratorType, ubrk_close, ubrk_first, ubrk_next, ubrk_open,
    ubrk_setUText, utext_close, utext_openUTF8,
};

fn boundaries(text: &str, kind: UBreakIteratorType) -> Option<Vec<usize>> {
    // ICU returns native UTF-8 byte offsets as signed 32-bit indices.
    let length = i32::try_from(text.len()).ok()?;
    let mut status = 0;
    // SAFETY: ICU borrows the live UTF-8 slice with an explicit length. It
    // accepts null text for the initial empty break iterator. setUText clones
    // the UText handle, so the original can close before iteration. The clone
    // closes with the iterator before the borrowed slice goes out of scope.
    unsafe {
        let input = utext_openUTF8(
            ptr::null_mut(),
            text.as_ptr(),
            i64::from(length),
            &mut status,
        );
        if input.is_null() {
            return None;
        }
        let iter = ubrk_open(kind, c"root".as_ptr().cast(), ptr::null(), 0, &mut status);
        if iter.is_null() {
            utext_close(input);
            return None;
        }
        ubrk_setUText(iter, input, &mut status);
        utext_close(input);
        let result = (status <= 0).then(|| {
            let mut offsets = Vec::new();
            let mut position = ubrk_first(iter);
            while position >= 0 {
                offsets.push(position as usize);
                position = ubrk_next(iter);
            }
            offsets
        });
        ubrk_close(iter);
        result
    }
}

pub(super) fn word_boundaries(text: &str) -> Option<Vec<usize>> {
    boundaries(text, UBRK_WORD)
}

/// Parley already handles ordinary Unicode line breaks. Supply the missing
/// soft wrap opportunities inside runs of complex-context characters. A style
/// covering just the following character adds a break at its leading edge.
pub(super) fn dictionary_line_breaks(text: &str) -> Vec<Range<usize>> {
    let properties = CodePointMapData::<LineBreak>::new();
    let complex = |ch| properties.get(ch) == LineBreak::ComplexContext;
    if text.is_ascii() || !text.chars().any(complex) {
        return Vec::new();
    }

    let mut ranges = Vec::new();
    for offset in boundaries(text, UBRK_LINE).unwrap_or_default() {
        let Some((before, after)) = text.split_at_checked(offset) else {
            continue;
        };
        if before.chars().next_back().is_some_and(complex)
            && let Some(next) = after.chars().next()
            && complex(next)
        {
            ranges.push(offset..offset + next.len_utf8());
        }
    }
    ranges
}
