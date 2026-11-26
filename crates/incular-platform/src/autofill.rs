//! Flutter-compatible autofill configuration and tagged input messages.

use std::fmt;

use serde_json::{Map, Value, json};

use crate::{
    TextInputClientId,
    message_codecs::{JsonMethodCodec, JsonMethodCodecError, MethodCall},
};

/// The action when the outermost retained autofill group is disposed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum AutofillContextAction {
    #[default]
    Commit,
    Cancel,
}

impl AutofillContextAction {
    #[must_use]
    pub const fn should_save(self) -> bool {
        matches!(self, Self::Commit)
    }
}

/// The iOS/macOS native keyboard policy differs from the browser and
/// other hosts. The web always uses [Self::Other].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AutofillKeyboardPlatform {
    Apple,
    Other,
}

impl AutofillKeyboardPlatform {
    #[must_use]
    pub const fn current() -> Self {
        if cfg!(any(target_os = "ios", target_os = "macos")) {
            Self::Apple
        } else {
            Self::Other
        }
    }
}

/// Flutter's EditableText keyboard inference. Only the first hint can
/// affect it, and explicitly configured keyboards take precedence in
/// the caller.
#[must_use]
pub fn infer_autofill_keyboard(
    platform: AutofillKeyboardPlatform,
    first_hint: Option<&str>,
    multiline: bool,
) -> crate::TextInputType {
    use crate::TextInputType::{
        self, DateTime, Email, Multiline, Name, Phone, StreetAddress, Text, Url,
    };
    let Some(hint) = first_hint else {
        return if multiline { Multiline } else { Text };
    };
    if platform == AutofillKeyboardPlatform::Apple {
        let apple = match hint {
            "addressCity"
            | "addressCityAndState"
            | "addressState"
            | "countryName"
            | "familyName"
            | "fullStreetAddress"
            | "givenName"
            | "jobTitle"
            | "location"
            | "middleName"
            | "name"
            | "namePrefix"
            | "nameSuffix"
            | "nickname"
            | "postalCode"
            | "streetAddressLine1"
            | "streetAddressLine2"
            | "sublocality"
            | "telephoneNumber" => Some(Name),
            "creditCardNumber" | "oneTimeCode" => Some(TextInputType::NUMBER),
            "email" => Some(Email),
            "emailOTPCode" | "newPassword" | "newUsername" | "organizationName" | "password"
            | "username" => Some(Text),
            "url" => Some(Url),
            _ => None,
        };
        if let Some(apple) = apple {
            return apple;
        }
    }
    if multiline {
        return Multiline;
    }
    match hint {
        "addressCity"
        | "addressCityAndState"
        | "addressState"
        | "fullStreetAddress"
        | "location"
        | "postalAddress"
        | "postalAddressExtended"
        | "streetAddressLevel1"
        | "streetAddressLevel2"
        | "streetAddressLevel3"
        | "streetAddressLevel4"
        | "streetAddressLine1"
        | "streetAddressLine2"
        | "streetAddressLine3"
        | "sublocality" => StreetAddress,
        "birthday"
        | "birthdayDay"
        | "birthdayMonth"
        | "birthdayYear"
        | "creditCardExpirationDate"
        | "creditCardExpirationDay"
        | "creditCardExpirationMonth"
        | "creditCardExpirationYear" => DateTime,
        "countryCode"
        | "creditCardNumber"
        | "creditCardSecurityCode"
        | "postalAddressExtendedPostalCode"
        | "postalCode" => TextInputType::NUMBER,
        "creditCardFamilyName"
        | "creditCardGivenName"
        | "creditCardMiddleName"
        | "creditCardName"
        | "familyName"
        | "givenName"
        | "middleInitial"
        | "middleName"
        | "name"
        | "namePrefix"
        | "nameSuffix" => Name,
        "email" => Email,
        "impp" | "url" => Url,
        "telephoneNumber"
        | "telephoneNumberAreaCode"
        | "telephoneNumberCountryCode"
        | "telephoneNumberDevice"
        | "telephoneNumberExtension"
        | "telephoneNumberLocal"
        | "telephoneNumberLocalPrefix"
        | "telephoneNumberLocalSuffix"
        | "telephoneNumberNational" => Phone,
        "transactionAmount" => TextInputType::number_with_options(false, true),
        _ => Text,
    }
}

/// The Flutter wire affinity of a UTF-16 text selection.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum AutofillSelectionAffinity {
    Upstream,
    #[default]
    Downstream,
}

/// A snapshot in the platform's UTF-16 coordinates. A missing selection
/// or composing range is the pair `(-1, -1)`.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct AutofillEditingValue {
    pub text: String,
    pub selection: (i64, i64),
    pub selection_affinity: AutofillSelectionAffinity,
    pub selection_is_directional: bool,
    pub composing: (i64, i64),
}

impl Default for AutofillEditingValue {
    fn default() -> Self {
        Self::new("")
    }
}

impl fmt::Debug for AutofillEditingValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AutofillEditingValue")
            .field("text", &"[redacted]")
            .field("selection", &self.selection)
            .field("selection_affinity", &self.selection_affinity)
            .field("selection_is_directional", &self.selection_is_directional)
            .field("composing", &self.composing)
            .finish()
    }
}

impl AutofillEditingValue {
    #[must_use]
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            selection: (-1, -1),
            selection_affinity: AutofillSelectionAffinity::Downstream,
            selection_is_directional: false,
            composing: (-1, -1),
        }
    }

    pub fn to_json(&self) -> Result<Value, AutofillMessageError> {
        self.validate()?;
        Ok(json!({
            "text": self.text,
            "selectionBase": self.selection.0,
            "selectionExtent": self.selection.1,
            "selectionAffinity": match self.selection_affinity {
                AutofillSelectionAffinity::Upstream => "TextAffinity.upstream",
                AutofillSelectionAffinity::Downstream => "TextAffinity.downstream",
            },
            "selectionIsDirectional": self.selection_is_directional,
            "composingBase": self.composing.0,
            "composingExtent": self.composing.1,
        }))
    }

    pub fn from_json(value: &Value) -> Result<Self, AutofillMessageError> {
        use AutofillMessageError::InvalidField;
        let text = value
            .get("text")
            .and_then(Value::as_str)
            .ok_or(InvalidField("text"))?;
        let offset = |field| match value.get(field) {
            None | Some(Value::Null) => Ok(-1),
            Some(value) => value.as_i64().ok_or(InvalidField(field)),
        };
        let selection_affinity = match value.get("selectionAffinity") {
            None | Some(Value::Null) => AutofillSelectionAffinity::Downstream,
            Some(Value::String(value)) if value == "TextAffinity.upstream" => {
                AutofillSelectionAffinity::Upstream
            }
            Some(Value::String(_)) => AutofillSelectionAffinity::Downstream,
            _ => return Err(InvalidField("selectionAffinity")),
        };
        let selection_is_directional = match value.get("selectionIsDirectional") {
            None | Some(Value::Null) => false,
            Some(Value::Bool(value)) => *value,
            _ => return Err(InvalidField("selectionIsDirectional")),
        };
        let state = Self {
            text: text.to_owned(),
            selection: (offset("selectionBase")?, offset("selectionExtent")?),
            selection_affinity,
            selection_is_directional,
            composing: (offset("composingBase")?, offset("composingExtent")?),
        };
        state.validate()?;
        Ok(state)
    }

    fn validate(&self) -> Result<(), AutofillMessageError> {
        let units = self.text.encode_utf16().count();
        let valid = |pair: (i64, i64)| {
            pair == (-1, -1)
                || [pair.0, pair.1]
                    .into_iter()
                    .all(|offset| usize::try_from(offset).is_ok_and(|offset| offset <= units))
        };
        if !valid(self.selection) {
            return Err(AutofillMessageError::InvalidField("selection range"));
        }
        if !valid(self.composing) || self.composing.0 > self.composing.1 {
            return Err(AutofillMessageError::InvalidField("composing range"));
        }
        Ok(())
    }
}

/// The Flutter autofill section of a text input configuration. An empty
/// hint list is enabled; a disabled configuration has no wire section.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct AutofillConfiguration {
    pub enabled: bool,
    pub unique_identifier: String,
    pub autofill_hints: Vec<String>,
    pub current_editing_value: AutofillEditingValue,
    pub hint_text: Option<String>,
}

impl AutofillConfiguration {
    #[must_use]
    pub fn new(
        unique_identifier: impl Into<String>,
        autofill_hints: impl IntoIterator<Item = impl Into<String>>,
        current_editing_value: AutofillEditingValue,
    ) -> Self {
        Self {
            enabled: true,
            unique_identifier: unique_identifier.into(),
            autofill_hints: autofill_hints.into_iter().map(Into::into).collect(),
            current_editing_value,
            hint_text: None,
        }
    }

    #[must_use]
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            unique_identifier: String::new(),
            autofill_hints: Vec::new(),
            current_editing_value: AutofillEditingValue::default(),
            hint_text: None,
        }
    }

    #[must_use]
    pub fn hint_text(mut self, hint: impl Into<String>) -> Self {
        self.hint_text = Some(hint.into());
        self
    }

    pub fn to_json(&self) -> Result<Option<Value>, AutofillMessageError> {
        if !self.enabled {
            return Ok(None);
        }
        let mut value = Map::from_iter([
            ("uniqueIdentifier".into(), json!(self.unique_identifier)),
            ("hints".into(), json!(self.autofill_hints)),
            ("editingValue".into(), self.current_editing_value.to_json()?),
        ]);
        if let Some(hint) = &self.hint_text {
            value.insert("hintText".into(), Value::String(hint.clone()));
        }
        Ok(Some(Value::Object(value)))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AutofillMessageError {
    #[error("malformed autofill method call")]
    Codec(#[from] JsonMethodCodecError),
    #[error("malformed autofill message field: {0}")]
    InvalidField(&'static str),
}

/// One field's complete text input options and its autofill snapshot.
#[derive(Clone)]
pub struct AutofillFieldConfiguration {
    pub input: Map<String, Value>,
    pub autofill: AutofillConfiguration,
}

impl AutofillFieldConfiguration {
    fn to_json(&self) -> Result<Map<String, Value>, AutofillMessageError> {
        let mut cfg = self.input.clone();
        if let Some(autofill) = self.autofill.to_json()? {
            cfg.insert("autofill".into(), autofill);
        } else {
            cfg.remove("autofill");
        }
        Ok(cfg)
    }
}

/// Ordered configuration of one attached autofill scope. The embedder
/// drops or replaces this scope when the active input connection changes.
#[derive(Clone, Default)]
pub struct FlutterAutofillScope {
    clients: Vec<AutofillFieldConfiguration>,
}

impl FlutterAutofillScope {
    #[must_use]
    pub fn get(&self, identifier: &str) -> Option<&AutofillFieldConfiguration> {
        self.clients
            .iter()
            .find(|client| client.autofill.unique_identifier == identifier)
    }

    /// Registers a field without replacing an existing client for the tag.
    pub fn register(&mut self, field: AutofillFieldConfiguration) -> bool {
        if self.get(&field.autofill.unique_identifier).is_some() {
            return false;
        }
        self.clients.push(field);
        true
    }

    /// Removes a field, preserving the relative order of remaining fields.
    pub fn unregister(&mut self, identifier: &str) -> bool {
        let Some(index) = self
            .clients
            .iter()
            .position(|client| client.autofill.unique_identifier == identifier)
        else {
            return false;
        };
        self.clients.remove(index);
        true
    }

    /// Encodes the trigger and the ordered enabled `fields` list.
    pub fn encode_set_client(
        &self,
        client: TextInputClientId,
        trigger: &AutofillFieldConfiguration,
    ) -> Result<Vec<u8>, AutofillMessageError> {
        let mut config = trigger.to_json()?;
        let fields = self
            .clients
            .iter()
            .filter(|field| field.autofill.enabled)
            .map(AutofillFieldConfiguration::to_json)
            .collect::<Result<Vec<_>, _>>()?;
        config.insert("fields".into(), json!(fields));
        Ok(JsonMethodCodec::encode_method_call(&MethodCall {
            method: "TextInput.setClient".into(),
            arguments: json!([client.get(), config]),
        })?)
    }

    /// Encodes the explicit save/cancel choice of a finishing group.
    pub fn encode_finish(should_save: bool) -> Result<Vec<u8>, AutofillMessageError> {
        Ok(JsonMethodCodec::encode_method_call(&MethodCall {
            method: "TextInput.finishAutofillContext".into(),
            arguments: Value::Bool(should_save),
        })?)
    }

    /// Delivers tagged values to enabled fields in this active scope, even
    /// when a tag is not the field that owns the active client number. All
    /// values are validated before applying any of them.
    pub fn handle_tagged_update(
        &self,
        data: &[u8],
        mut apply: impl FnMut(&str, AutofillEditingValue),
    ) -> Result<bool, AutofillMessageError> {
        let Some(values) = Self::decode_tagged_update(data)? else {
            return Ok(false);
        };
        for (tag, value) in values {
            if self.get(&tag).is_some_and(|field| field.autofill.enabled) {
                apply(&tag, value);
            }
        }
        Ok(true)
    }

    /// Decodes a tagged batch without collecting or exposing the current
    /// text of any already mounted field.
    pub fn decode_tagged_update(
        data: &[u8],
    ) -> Result<Option<Vec<(String, AutofillEditingValue)>>, AutofillMessageError> {
        let call = JsonMethodCodec::decode_method_call(data)?;
        if call.method != "TextInputClient.updateEditingStateWithTag" {
            return Ok(None);
        }
        let fields = call
            .arguments
            .as_array()
            .and_then(|args| args.get(1))
            .and_then(Value::as_object)
            .ok_or(AutofillMessageError::InvalidField("tagged editing values"))?;
        let values = fields
            .iter()
            .map(|(tag, value)| {
                AutofillEditingValue::from_json(value).map(|value| (tag.clone(), value))
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Some(values))
    }
}
