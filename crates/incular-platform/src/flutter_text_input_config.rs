//! Flutter text-input configuration on its JSON channel, not a native IME state.

use incular_config::{Brightness, Locale};
use serde_json::{Map, Value, json};

use crate::{SmartDashesType, SmartQuotesType, TextCapitalization, TextInputAction, TextInputType};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FlutterTextInputConfiguration {
    pub view_id: Option<i64>,
    pub input_type: TextInputType,
    pub read_only: bool,
    pub obscure_text: bool,
    pub autocorrect: bool,
    pub smart_dashes_type: Option<SmartDashesType>,
    pub smart_quotes_type: Option<SmartQuotesType>,
    pub enable_suggestions: bool,
    pub enable_interactive_selection: bool,
    pub action_label: Option<String>,
    pub input_action: TextInputAction,
    pub text_capitalization: TextCapitalization,
    pub keyboard_appearance: Brightness,
    pub enable_ime_personalized_learning: bool,
    pub allowed_mime_types: Vec<String>,
    pub autofill: Option<Map<String, Value>>,
    pub enable_delta_model: bool,
    pub hint_locales: Option<Vec<Locale>>,
    pub enable_inline_prediction: Option<bool>,
}

impl Default for FlutterTextInputConfiguration {
    fn default() -> Self {
        Self {
            view_id: None,
            input_type: TextInputType::Text,
            read_only: false,
            obscure_text: false,
            autocorrect: true,
            smart_dashes_type: None,
            smart_quotes_type: None,
            enable_suggestions: true,
            enable_interactive_selection: true,
            action_label: None,
            input_action: TextInputAction::Done,
            text_capitalization: TextCapitalization::None,
            keyboard_appearance: Brightness::Light,
            enable_ime_personalized_learning: true,
            allowed_mime_types: Vec::new(),
            autofill: None,
            enable_delta_model: false,
            hint_locales: Some(Vec::new()),
            enable_inline_prediction: None,
        }
    }
}

impl FlutterTextInputConfiguration {
    #[must_use]
    pub fn to_json(&self) -> Map<String, Value> {
        let dashes = self.smart_dashes_type.unwrap_or(if self.obscure_text {
            SmartDashesType::Disabled
        } else {
            SmartDashesType::Enabled
        });
        let quotes = self.smart_quotes_type.unwrap_or(if self.obscure_text {
            SmartQuotesType::Disabled
        } else {
            SmartQuotesType::Enabled
        });
        let action = match self.input_action {
            TextInputAction::Unspecified | TextInputAction::Done => "done",
            TextInputAction::None => "none",
            TextInputAction::Go => "go",
            TextInputAction::Search => "search",
            TextInputAction::Send => "send",
            TextInputAction::Next => "next",
            TextInputAction::Previous => "previous",
            TextInputAction::Newline => "newline",
        };
        let capitalization = match self.text_capitalization {
            TextCapitalization::None => "none",
            TextCapitalization::Characters => "characters",
            TextCapitalization::Words => "words",
            TextCapitalization::Sentences => "sentences",
        };
        let keyboard = match self.keyboard_appearance {
            Brightness::Light => "Brightness.light",
            Brightness::Dark => "Brightness.dark",
        };
        let hint_locales = self
            .hint_locales
            .as_ref()
            .map(|locales| locales.iter().map(ToString::to_string).collect::<Vec<_>>());
        let Value::Object(mut fields) = json!({
            "viewId": self.view_id,
            "inputType": self.input_type.to_flutter_json(),
            "readOnly": self.read_only,
            "obscureText": self.obscure_text,
            "autocorrect": self.autocorrect,
            "smartDashesType": dashes.flutter_index(),
            "smartQuotesType": quotes.flutter_index(),
            "enableSuggestions": self.enable_suggestions,
            "enableInteractiveSelection": self.enable_interactive_selection,
            "actionLabel": self.action_label,
            "inputAction": format!("TextInputAction.{action}"),
            "textCapitalization": format!("TextCapitalization.{capitalization}"),
            "keyboardAppearance": keyboard,
            "enableIMEPersonalizedLearning": self.enable_ime_personalized_learning,
            "contentCommitMimeTypes": self.allowed_mime_types,
            "enableDeltaModel": self.enable_delta_model,
            "hintLocales": hint_locales,
            "enableInlinePrediction": self.enable_inline_prediction,
        }) else {
            unreachable!("text input configuration is an object")
        };
        if let Some(autofill) = &self.autofill {
            fields.insert("autofill".into(), Value::Object(autofill.clone()));
        }
        fields
    }
}

impl TextInputType {
    #[must_use]
    pub fn to_flutter_json(self) -> Value {
        let (signed, decimal, password) = match self {
            Self::Number { signed, decimal } => (Some(signed), Some(decimal), Some(false)),
            Self::NumberPassword { signed, decimal } => (Some(signed), Some(decimal), Some(true)),
            _ => (None, None, None),
        };
        json!({
            "name": self.flutter_name(),
            "signed": signed,
            "decimal": decimal,
            "password": password,
        })
    }
}
