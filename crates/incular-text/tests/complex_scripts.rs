//! Line breaking for scripts written without spaces between words.
//!
//! Thai, Lao, Khmer and Burmese break between words, which only a
//! dictionary or model can find; Chinese and Japanese break between
//! characters. Native targets carry that data, so an application needs no
//! dependency or feature of its own for these scripts to wrap.
#![cfg(not(target_arch = "wasm32"))]

use icu_segmenter::{WordSegmenter, options::WordBreakInvariantOptions};
use incular_text::{TextAlign, TextEngine, TextStyle};

const THAI: &str = "ภาษาไทยสำหรับค้นหาภาพยนตร์และรายการโทรทัศน์";
const JAPANESE: &str = "千と千尋の神隠し。日本語の映画とテレビ番組を探します。";
const CHINESE: &str = "你好世界。浏览电影和电视剧，发现新的精彩故事。";

/// Byte offsets where each laid-out line after the first starts.
fn line_starts(text: &str, max_width: f32) -> Vec<usize> {
    let mut engine = TextEngine::new();
    let layout = engine.layout(
        text,
        &TextStyle::default(),
        Some(max_width),
        TextAlign::Start,
    );
    assert!(layout.metrics.size.height > 0.);
    for line in layout.lines.iter() {
        assert!(text.is_char_boundary(line.start) && text.is_char_boundary(line.end));
    }
    layout.lines.iter().skip(1).map(|line| line.start).collect()
}

#[test]
fn thai_wraps_between_words() {
    let words: Vec<usize> = WordSegmenter::new_auto(WordBreakInvariantOptions::default())
        .segment_str(THAI)
        .collect();
    assert!(
        words.len() > 4,
        "the word segmenter finds words inside unspaced Thai: {words:?}"
    );
    let starts = line_starts(THAI, 80.);
    assert!(!starts.is_empty(), "unspaced Thai wraps at a narrow width");
    for start in &starts {
        assert!(
            words.contains(start),
            "line starts at byte {start}, inside a word; word boundaries are {words:?}"
        );
    }
}

#[test]
fn chinese_and_japanese_wrap_between_characters() {
    for text in [JAPANESE, CHINESE] {
        assert!(
            line_starts(text, 80.).len() > 1,
            "ideographic text wraps: {text}"
        );
    }
}
