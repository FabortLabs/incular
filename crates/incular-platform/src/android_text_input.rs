//! Android input-type bits derived from a Flutter-compatible configuration.

use crate::{TextCapitalization, TextInputConfiguration, TextInputType};

/// Android's EditorInfo input type for this text input client.
#[must_use]
pub fn flutter_android_input_type(configuration: &TextInputConfiguration) -> i32 {
    const CLASS_TEXT: i32 = 0x1;
    const CLASS_NUMBER: i32 = 0x2;
    const CLASS_PHONE: i32 = 0x3;
    const CLASS_DATETIME: i32 = 0x4;
    const VARIATION_NUMBER_PASSWORD: i32 = 0x10;
    const FLAG_NUMBER_SIGNED: i32 = 0x1000;
    const FLAG_NUMBER_DECIMAL: i32 = 0x2000;
    const VARIATION_URI: i32 = 0x10;
    const VARIATION_EMAIL_ADDRESS: i32 = 0x20;
    const VARIATION_PERSON_NAME: i32 = 0x60;
    const VARIATION_POSTAL_ADDRESS: i32 = 0x70;
    const VARIATION_PASSWORD: i32 = 0x80;
    const VARIATION_VISIBLE_PASSWORD: i32 = 0x90;
    const FLAG_CAP_CHARACTERS: i32 = 0x1000;
    const FLAG_CAP_WORDS: i32 = 0x2000;
    const FLAG_CAP_SENTENCES: i32 = 0x4000;
    const FLAG_AUTO_CORRECT: i32 = 0x8000;
    const FLAG_MULTI_LINE: i32 = 0x2_0000;
    const FLAG_NO_SUGGESTIONS: i32 = 0x8_0000;
    let mut bits = match configuration.input_type {
        TextInputType::Number { signed, decimal }
        | TextInputType::NumberPassword { signed, decimal } => {
            let mut bits = CLASS_NUMBER;
            if configuration.input_type.is_password_keyboard() {
                bits |= VARIATION_NUMBER_PASSWORD;
            }
            if signed {
                bits |= FLAG_NUMBER_SIGNED;
            }
            if decimal {
                bits |= FLAG_NUMBER_DECIMAL;
            }
            return bits;
        }
        TextInputType::Phone => return CLASS_PHONE,
        TextInputType::DateTime => return CLASS_DATETIME,
        TextInputType::None => return 0, // TYPE_NULL
        TextInputType::Name => CLASS_TEXT | VARIATION_PERSON_NAME,
        TextInputType::StreetAddress => CLASS_TEXT | VARIATION_POSTAL_ADDRESS,
        TextInputType::Email | TextInputType::Twitter => CLASS_TEXT | VARIATION_EMAIL_ADDRESS,
        TextInputType::Url | TextInputType::WebSearch => CLASS_TEXT | VARIATION_URI,
        TextInputType::Text | TextInputType::Multiline | TextInputType::Password => CLASS_TEXT,
    };
    if configuration.multiline {
        bits |= FLAG_MULTI_LINE;
    }
    let traits = &configuration.traits;
    if configuration.obscure_text || configuration.input_type == TextInputType::Password {
        bits |= FLAG_NO_SUGGESTIONS | VARIATION_PASSWORD;
    } else {
        if traits.autocorrect {
            bits |= FLAG_AUTO_CORRECT;
        }
        if !traits.enable_suggestions {
            // Flutter's workaround: keyboards ignore NO_SUGGESTIONS alone.
            bits |= FLAG_NO_SUGGESTIONS | VARIATION_VISIBLE_PASSWORD;
        }
    }
    bits | match traits.text_capitalization {
        TextCapitalization::Characters => FLAG_CAP_CHARACTERS,
        TextCapitalization::Words => FLAG_CAP_WORDS,
        TextCapitalization::Sentences => FLAG_CAP_SENTENCES,
        TextCapitalization::None => 0,
    }
}
