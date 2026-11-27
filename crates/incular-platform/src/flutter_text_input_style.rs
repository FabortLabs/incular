//! Flutter's style snapshot for system text input controls.

use std::hash::{Hash, Hasher};

use incular_config::TextDirection;
use serde_json::{Value, json};

use crate::{FlutterJsonMethodChannel, FlutterPlatformChannelError, FlutterPlatformMessenger};

/// The Flutter alignment index, not an Incular layout enum's discriminant.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FlutterTextInputAlignment {
    Left = 0,
    Right = 1,
    Center = 2,
    Justify = 3,
    Start = 4,
    End = 5,
}

/// The index into Flutter's ordered array of standard font weights.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FlutterTextInputWeight {
    W100 = 0,
    W200 = 1,
    W300 = 2,
    W400 = 3,
    W500 = 4,
    W600 = 5,
    W700 = 6,
    W800 = 7,
    W900 = 8,
}

impl FlutterTextInputWeight {
    pub const NORMAL: Self = Self::W400;
    pub const BOLD: Self = Self::W700;
}

/// Style metadata for a Flutter-compatible text input client.
#[derive(Clone, Debug, PartialEq)]
pub struct FlutterTextInputStyle {
    pub font_family: Option<String>,
    pub font_size: Option<f64>,
    pub font_weight: Option<FlutterTextInputWeight>,
    pub text_direction: TextDirection,
    pub text_align: FlutterTextInputAlignment,
    pub letter_spacing: Option<f64>,
    pub word_spacing: Option<f64>,
    pub line_height: Option<f64>,
}

impl FlutterTextInputStyle {
    #[must_use]
    pub const fn new(text_direction: TextDirection, text_align: FlutterTextInputAlignment) -> Self {
        Self {
            font_family: None,
            font_size: None,
            font_weight: None,
            text_direction,
            text_align,
            letter_spacing: None,
            word_spacing: None,
            line_height: None,
        }
    }

    /// The eight fields sent on Flutter's platform channel. Missing optional
    /// values are explicit JSON nulls. Nonfinite values cannot be JSON numbers.
    pub fn to_json(&self) -> Result<Value, InvalidFlutterTextInputStyleNumber> {
        for (field, value) in [
            ("fontSize", self.font_size),
            ("letterSpacing", self.letter_spacing),
            ("wordSpacing", self.word_spacing),
            ("lineHeight", self.line_height),
        ] {
            if value.is_some_and(|value| !value.is_finite()) {
                return Err(InvalidFlutterTextInputStyleNumber(field));
            }
        }
        Ok(json!({
            "fontFamily": self.font_family,
            "fontSize": self.font_size,
            "fontWeightIndex": self.font_weight.map(|weight| weight as u8),
            "textAlignIndex": self.text_align as u8,
            "textDirectionIndex": match self.text_direction {
                TextDirection::Rtl => 0,
                TextDirection::Ltr => 1,
            },
            "letterSpacing": self.letter_spacing,
            "wordSpacing": self.word_spacing,
            "lineHeight": self.line_height,
        }))
    }

    /// Sends the style to the host, without changing the focused attachment.
    pub async fn send<M: FlutterPlatformMessenger>(
        &self,
        messenger: &mut M,
    ) -> Result<(), FlutterTextInputStyleSendError<M::Error>> {
        FlutterJsonMethodChannel::new("flutter/textinput")
            .invoke(messenger, "TextInput.setStyle", self.to_json()?)
            .await?;
        Ok(())
    }
    /// Reports a failure to a local diagnostics sink, keeping the original
    /// platform exception and the operation that produced it.
    pub async fn send_reporting<M: FlutterPlatformMessenger>(
        &self,
        messenger: &mut M,
        report: impl FnOnce(FlutterTextInputStyleSendError<M::Error>),
    ) {
        if let Err(error) = self.send(messenger).await {
            report(error);
        }
    }
}

impl Hash for FlutterTextInputStyle {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let bits =
            |value: Option<f64>| value.map(|value| if value == 0. { 0 } else { value.to_bits() });
        self.font_family.hash(state);
        bits(self.font_size).hash(state);
        self.font_weight.hash(state);
        self.text_direction.hash(state);
        self.text_align.hash(state);
        bits(self.letter_spacing).hash(state);
        bits(self.word_spacing).hash(state);
        bits(self.line_height).hash(state);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
#[error("nonfinite number in Flutter text input style field {0}")]
pub struct InvalidFlutterTextInputStyleNumber(pub &'static str);

#[derive(Debug, thiserror::Error)]
pub enum FlutterTextInputStyleSendError<E> {
    #[error(transparent)]
    InvalidNumber(#[from] InvalidFlutterTextInputStyleNumber),
    #[error("while updating text input style: {0}")]
    Channel(#[from] FlutterPlatformChannelError<E>),
}
