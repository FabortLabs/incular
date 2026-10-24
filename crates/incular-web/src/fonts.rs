//! Fonts for scripts the bundled fonts do not cover, fetched on demand.
//!
//! A page has no system fonts to fall back to. When a layout draws a
//! character with the missing glyph, the Noto font for that character's
//! script is fetched once and registered with the text engines, and the
//! application lays its text out again, as in Flutter's web engine.

/// Where a browser application fetches fonts for characters its registered
/// fonts cannot draw. Set it with [`set_font_fallback`] before running.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub enum FontFallback {
    /// Noto fonts from the jsDelivr CDN.
    #[default]
    NotoCdn,
    /// The same Noto font files, by file name, from this base URL (for
    /// example a copy on the application's own host): `{base}/{file}`.
    /// [`FALLBACK_FONT_FILES`] lists every file that may be requested.
    Mirror(String),
    /// Never fetch fonts; characters without a glyph stay blank boxes.
    Disabled,
}

/// Every font file [`FontFallback::Mirror`] may request.
pub const FALLBACK_FONT_FILES: &[&str] = &{
    let mut files = [""; FONTS.len()];
    let mut index = 0;
    while index < FONTS.len() {
        files[index] = FONTS[index].file;
        index += 1;
    }
    files
};

/// Sets how fonts for uncovered characters are fetched, for applications
/// started afterwards on this thread. Off the browser target this has no
/// effect.
pub fn set_font_fallback(fallback: FontFallback) {
    #[cfg(target_arch = "wasm32")]
    page::FALLBACK.with_borrow_mut(|current| *current = fallback);
    #[cfg(not(target_arch = "wasm32"))]
    let _ = fallback;
}

/// The fallback font file, one of [`FALLBACK_FONT_FILES`], that covers
/// `character`, for a page whose language is `language` (a BCP 47 tag, which
/// picks the Chinese, Japanese or Korean font for shared ideographs).
#[must_use]
pub fn fallback_font_file(character: char, language: &str) -> Option<&'static str> {
    font_for(character, cjk_for_language(language)).map(|font| FONTS[font].file)
}

struct FallbackFont {
    file: &'static str,
    /// Path of `file` on the jsDelivr CDN, which only a page fetches.
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    cdn_path: &'static str,
}

macro_rules! noto {
    ($family:literal) => {
        FallbackFont {
            file: concat!($family, "-Regular.ttf"),
            cdn_path: concat!(
                "notofonts/notofonts.github.io/fonts/",
                $family,
                "/hinted/ttf/",
                $family,
                "-Regular.ttf"
            ),
        }
    };
}

macro_rules! noto_cjk {
    ($region:literal) => {
        FallbackFont {
            file: concat!("NotoSans", $region, "-Regular.otf"),
            cdn_path: concat!(
                "notofonts/noto-cjk@main/Sans/SubsetOTF/",
                $region,
                "/NotoSans",
                $region,
                "-Regular.otf"
            ),
        }
    };
}

const NOTO_SANS: usize = 0;
const CJK_SC: usize = 1;
const CJK_TC: usize = 2;
const CJK_JP: usize = 3;
const CJK_KR: usize = 4;
const EMOJI: usize = 5;
const SYMBOLS: usize = 6;
const MATH: usize = 7;

const FONTS: [FallbackFont; 39] = [
    noto!("NotoSans"),
    noto_cjk!("SC"),
    noto_cjk!("TC"),
    noto_cjk!("JP"),
    noto_cjk!("KR"),
    FallbackFont {
        file: "NotoEmoji-Regular.ttf",
        cdn_path: "google/fonts@main/ofl/notoemoji/NotoEmoji%5Bwght%5D.ttf",
    },
    noto!("NotoSansSymbols2"),
    noto!("NotoSansMath"),
    noto!("NotoSansArabic"),
    noto!("NotoSansHebrew"),
    noto!("NotoSansArmenian"),
    noto!("NotoSansSyriac"),
    noto!("NotoSansThaana"),
    noto!("NotoSansDevanagari"),
    noto!("NotoSansBengali"),
    noto!("NotoSansGurmukhi"),
    noto!("NotoSansGujarati"),
    noto!("NotoSansOriya"),
    noto!("NotoSansTamil"),
    noto!("NotoSansTelugu"),
    noto!("NotoSansKannada"),
    noto!("NotoSansMalayalam"),
    noto!("NotoSansSinhala"),
    noto!("NotoSansThai"),
    noto!("NotoSansLao"),
    noto!("NotoSerifTibetan"),
    noto!("NotoSansMyanmar"),
    noto!("NotoSansGeorgian"),
    noto!("NotoSansEthiopic"),
    noto!("NotoSansCherokee"),
    noto!("NotoSansKhmer"),
    noto!("NotoSansMongolian"),
    noto!("NotoSansNKo"),
    noto!("NotoSansTifinagh"),
    noto!("NotoSansCanadianAboriginal"),
    noto!("NotoSansJavanese"),
    noto!("NotoSansBalinese"),
    noto!("NotoSansSundanese"),
    noto!("NotoSansOlChiki"),
];

/// Font indices for blocks, by first code point; a code point belongs to the
/// last block starting at or before it. `None` blocks need no font or have
/// no Noto font here.
const BLOCKS: &[(u32, Option<usize>)] = &[
    (0x0000, Some(NOTO_SANS)),
    (0x0530, Some(10)),
    (0x0590, Some(9)),
    (0x0600, Some(8)),
    (0x0700, Some(11)),
    (0x0750, Some(8)),
    (0x0780, Some(12)),
    (0x07C0, Some(32)),
    (0x0800, None),
    (0x0870, Some(8)),
    (0x0900, Some(13)),
    (0x0980, Some(14)),
    (0x0A00, Some(15)),
    (0x0A80, Some(16)),
    (0x0B00, Some(17)),
    (0x0B80, Some(18)),
    (0x0C00, Some(19)),
    (0x0C80, Some(20)),
    (0x0D00, Some(21)),
    (0x0D80, Some(22)),
    (0x0E00, Some(23)),
    (0x0E80, Some(24)),
    (0x0F00, Some(25)),
    (0x1000, Some(26)),
    (0x10A0, Some(27)),
    (0x1100, Some(CJK_KR)),
    (0x1200, Some(28)),
    (0x13A0, Some(29)),
    (0x1400, Some(34)),
    (0x1680, None),
    (0x1780, Some(30)),
    (0x1800, Some(31)),
    (0x18B0, Some(34)),
    (0x1900, None),
    (0x1B00, Some(36)),
    (0x1B80, Some(37)),
    (0x1BC0, None),
    (0x1C50, Some(38)),
    (0x1C80, Some(NOTO_SANS)),
    (0x1C90, Some(27)),
    (0x1CC0, Some(37)),
    (0x1CD0, Some(13)),
    (0x1D00, Some(NOTO_SANS)),
    (0x2190, Some(SYMBOLS)),
    (0x2200, Some(MATH)),
    (0x2300, Some(SYMBOLS)),
    (0x2C00, Some(NOTO_SANS)),
    (0x2D00, Some(27)),
    (0x2D30, Some(33)),
    (0x2D80, Some(28)),
    (0x2DE0, Some(NOTO_SANS)),
    (0x2E00, Some(NOTO_SANS)),
    (0x2E80, Some(CJK_SC)),
    (0x3040, Some(CJK_JP)),
    (0x3100, Some(CJK_TC)),
    (0x3130, Some(CJK_KR)),
    (0x3190, Some(CJK_SC)),
    (0x31F0, Some(CJK_JP)),
    (0x3200, Some(CJK_SC)),
    (0xA000, None),
    (0xA640, Some(NOTO_SANS)),
    (0xA6A0, None),
    (0xA700, Some(NOTO_SANS)),
    (0xA800, None),
    (0xA8E0, Some(13)),
    (0xA900, None),
    (0xA980, Some(35)),
    (0xA9E0, Some(26)),
    (0xAA00, None),
    (0xAA60, Some(26)),
    (0xAA80, None),
    (0xAB00, Some(28)),
    (0xAB30, Some(NOTO_SANS)),
    (0xAB70, Some(29)),
    (0xABC0, None),
    (0xAC00, Some(CJK_KR)),
    (0xD7B0, Some(CJK_KR)),
    (0xD800, None),
    (0xF900, Some(CJK_SC)),
    (0xFB00, Some(NOTO_SANS)),
    (0xFB1D, Some(9)),
    (0xFB50, Some(8)),
    (0xFE00, None),
    (0xFE10, Some(CJK_SC)),
    (0xFE20, Some(NOTO_SANS)),
    (0xFE30, Some(CJK_SC)),
    (0xFE70, Some(8)),
    (0xFF00, Some(CJK_SC)),
    (0xFFF0, None),
    (0x1D400, Some(MATH)),
    (0x1D800, None),
    (0x1F000, Some(EMOJI)),
    (0x1FB00, Some(SYMBOLS)),
    (0x1FC00, None),
    (0x20000, Some(CJK_SC)),
    (0x30000, None),
];

/// Characters that are emoji by default even though their block is not.
fn is_emoji(character: char) -> bool {
    matches!(
        u32::from(character),
        0x231A..=0x231B
            | 0x23E9..=0x23F3
            | 0x23F8..=0x23FA
            | 0x25FD..=0x25FE
            | 0x2600..=0x27BF
            | 0x2B50
            | 0x2B55
    )
}

/// The font index for `character`, preferring the CJK region `cjk` for
/// unified ideographs.
fn font_for(character: char, cjk: usize) -> Option<usize> {
    if is_emoji(character) {
        return Some(EMOJI);
    }
    let code = u32::from(character);
    let block = BLOCKS
        .partition_point(|(start, _)| *start <= code)
        .checked_sub(1)?;
    BLOCKS[block]
        .1
        .map(|font| if font == CJK_SC { cjk } else { font })
}

/// The CJK font for unified ideographs, from a language tag.
fn cjk_for_language(language: &str) -> usize {
    let language = language.to_ascii_lowercase();
    if language.starts_with("ja") {
        CJK_JP
    } else if language.starts_with("ko") {
        CJK_KR
    } else if language.starts_with("zh-hant")
        || language.starts_with("zh-tw")
        || language.starts_with("zh-hk")
        || language.starts_with("zh-mo")
    {
        CJK_TC
    } else {
        CJK_SC
    }
}

#[cfg(target_arch = "wasm32")]
mod page {
    use std::{cell::RefCell, collections::HashSet};

    use wasm_bindgen::{JsCast as _, JsValue};

    use super::{FONTS, FontFallback, cjk_for_language, font_for};

    const NOTO_CDN: &str = "https://cdn.jsdelivr.net/gh/";

    thread_local! {
        pub(super) static FALLBACK: RefCell<FontFallback> = RefCell::new(FontFallback::default());
        /// Font indices requested so far, loaded or not.
        static REQUESTED: RefCell<HashSet<usize>> = RefCell::new(HashSet::new());
    }

    /// Listens for characters the text engines could not draw.
    pub(crate) fn install() {
        incular_text::set_missing_characters_listener(|| {
            // Layout is running; collect after it.
            wasm_bindgen_futures::spawn_local(async { load_missing().await });
        });
    }

    async fn load_missing() {
        let characters = incular_text::take_missing_characters();
        let fallback = FALLBACK.with_borrow(Clone::clone);
        if characters.is_empty() || fallback == FontFallback::Disabled {
            return;
        }
        let cjk = cjk_for_language(
            &web_sys::window()
                .and_then(|window| window.navigator().language())
                .unwrap_or_default(),
        );
        let mut fonts = characters
            .into_iter()
            .filter_map(|character| font_for(character, cjk))
            .filter(|font| REQUESTED.with_borrow_mut(|requested| requested.insert(*font)))
            .collect::<Vec<_>>();
        fonts.sort_unstable();
        fonts.dedup();
        for font in fonts {
            let font = &FONTS[font];
            let url = match &fallback {
                FontFallback::Mirror(base) => {
                    format!("{}/{}", base.trim_end_matches('/'), font.file)
                }
                _ => format!("{NOTO_CDN}{}", font.cdn_path),
            };
            wasm_bindgen_futures::spawn_local(async move {
                match fetch(&url).await {
                    Ok(bytes) => {
                        incular_text::register_application_font(bytes);
                        crate::wake_host();
                    }
                    Err(error) => {
                        web_sys::console::warn_2(
                            &JsValue::from_str(&format!("Incular could not load font {url}:")),
                            &error,
                        );
                    }
                }
            });
        }
    }

    async fn fetch(url: &str) -> Result<Vec<u8>, JsValue> {
        let window = web_sys::window().ok_or_else(|| JsValue::from_str("no window"))?;
        let response = wasm_bindgen_futures::JsFuture::from(window.fetch_with_str(url))
            .await?
            .dyn_into::<web_sys::Response>()?;
        if !response.ok() {
            return Err(JsValue::from_str(&format!("HTTP {}", response.status())));
        }
        let buffer = wasm_bindgen_futures::JsFuture::from(response.array_buffer()?).await?;
        Ok(js_sys::Uint8Array::new(&buffer).to_vec())
    }
}

#[cfg(target_arch = "wasm32")]
pub(crate) use page::install;
