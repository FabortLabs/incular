//! Word boundaries from the browser's `Intl.Segmenter`.
//!
//! The text engine's browser build leaves out ICU's dictionaries for
//! Chinese, Japanese and Southeast Asian scripts. The browser has its own,
//! so word selection and word-wise caret movement use them, as in Flutter's
//! web engine. Browsers without `Intl.Segmenter` keep the built-in UAX #29
//! segmentation.

use wasm_bindgen::{JsCast as _, JsValue};

pub(crate) fn install() {
    let Some(segmenter) = word_segmenter() else {
        return;
    };
    incular_text::set_word_segmenter(move |text| word_boundaries(&segmenter, text));
}

fn word_segmenter() -> Option<js_sys::Object> {
    let intl = js_sys::Reflect::get(&js_sys::global(), &JsValue::from_str("Intl")).ok()?;
    let constructor = js_sys::Reflect::get(&intl, &JsValue::from_str("Segmenter"))
        .ok()?
        .dyn_into::<js_sys::Function>()
        .ok()?;
    let options = js_sys::Object::new();
    js_sys::Reflect::set(
        &options,
        &JsValue::from_str("granularity"),
        &JsValue::from_str("word"),
    )
    .ok()?;
    // An undefined locale is the page's.
    js_sys::Reflect::construct(
        &constructor,
        &js_sys::Array::of2(&JsValue::UNDEFINED, &options),
    )
    .ok()?
    .dyn_into()
    .ok()
}

/// The UTF-8 offsets of `text`'s word boundaries, from the UTF-16 indices
/// the browser reports.
fn word_boundaries(segmenter: &js_sys::Object, text: &str) -> Option<Vec<usize>> {
    let segment = js_sys::Reflect::get(segmenter, &JsValue::from_str("segment"))
        .ok()?
        .dyn_into::<js_sys::Function>()
        .ok()?;
    let segments = segment.call1(segmenter, &JsValue::from_str(text)).ok()?;
    let index_key = JsValue::from_str("index");
    let mut boundaries = vec![0];
    let mut characters = text.chars();
    let (mut utf8, mut utf16) = (0, 0);
    for item in js_sys::try_iter(&segments).ok()?? {
        let start = js_sys::Reflect::get(&item.ok()?, &index_key)
            .ok()?
            .as_f64()? as usize;
        while utf16 < start {
            let character = characters.next()?;
            utf16 += character.len_utf16();
            utf8 += character.len_utf8();
        }
        if utf8 > 0 {
            boundaries.push(utf8);
        }
    }
    if !text.is_empty() {
        boundaries.push(text.len());
    }
    boundaries.dedup();
    Some(boundaries)
}
