//! Flutter's host-facing indirect handwriting registry.

use std::rc::Rc;

use incular_core::{Offset, Rect, Size};
use serde_json::{Value, json};

use crate::message_codecs::{JsonMethodCodec, JsonMethodCodecError};

/// A mounted element available to a host handwriting interaction.
pub trait FlutterScribbleClient {
    fn bounds(&self) -> Rect;

    fn is_in_scribble_rect(&self, rect: Rect) -> bool {
        self.bounds().intersects(rect)
    }

    fn on_scribble_focus(&self, position: Offset);
}

/// Local registry; registration lifetime follows the mounted elements.
#[derive(Default)]
pub struct FlutterScribbleRegistry {
    // The host walks targets in registration order, even when several
    // have the same bounds. Replacing an ID keeps its position.
    clients: Vec<(String, Rc<dyn FlutterScribbleClient>)>,
    in_progress: bool,
}

impl FlutterScribbleRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_element(
        &mut self,
        id: impl Into<String>,
        client: Rc<dyn FlutterScribbleClient>,
    ) {
        let id = id.into();
        if let Some((_, previous)) = self
            .clients
            .iter_mut()
            .find(|(existing, _)| *existing == id)
        {
            *previous = client;
        } else {
            self.clients.push((id, client));
        }
    }

    pub fn unregister_element(&mut self, id: &str) -> bool {
        let Some(position) = self.clients.iter().position(|(existing, _)| existing == id) else {
            return false;
        };
        self.clients.remove(position);
        true
    }

    #[must_use]
    pub const fn in_progress(&self) -> bool {
        self.in_progress
    }

    /// Decodes a handwriting message, before requiring a focused text
    /// attachment. A recognized void operation returns a JSON null value.
    pub fn handle_call(&mut self, bytes: &[u8]) -> Result<Option<Value>, FlutterScribbleError> {
        let call = JsonMethodCodec::decode_method_call(bytes)?;
        let arguments = || {
            call.arguments
                .as_array()
                .ok_or(FlutterScribbleError::InvalidArguments)
        };
        let result = match call.method.as_str() {
            "TextInputClient.scribbleInteractionBegan" => {
                self.in_progress = true;
                Value::Null
            }
            "TextInputClient.scribbleInteractionFinished" => {
                self.in_progress = false;
                Value::Null
            }
            "TextInputClient.focusElement" => {
                let arguments = arguments()?;
                let id = arguments
                    .first()
                    .and_then(Value::as_str)
                    .ok_or(FlutterScribbleError::InvalidArguments)?;
                if let Some((_, client)) = self.clients.iter().find(|(existing, _)| existing == id)
                {
                    client.on_scribble_focus(Offset::new(
                        coordinate(arguments.get(1))?,
                        coordinate(arguments.get(2))?,
                    ));
                }
                Value::Null
            }
            "TextInputClient.requestElementsInRect" => {
                let arguments = arguments()?;
                let values = arguments
                    .iter()
                    .map(|value| coordinate(Some(value)))
                    .collect::<Result<Vec<_>, _>>()?;
                let [x, y, width, height, ..] = values.as_slice() else {
                    return Err(FlutterScribbleError::InvalidArguments);
                };
                let size = Size::maybe_new(*width, *height)
                    .ok_or(FlutterScribbleError::InvalidArguments)?;
                let query = Rect::from_origin_size(Offset::new(*x, *y), size);
                Value::Array(
                    self.clients
                        .iter()
                        .filter_map(|(id, client)| {
                            if !client.is_in_scribble_rect(query) {
                                return None;
                            }
                            let bounds = client.bounds();
                            let components = [
                                bounds.origin.x,
                                bounds.origin.y,
                                bounds.size.width,
                                bounds.size.height,
                            ];
                            if components.iter().all(|value| *value == 0.)
                                || !components.iter().all(|value| value.is_finite())
                            {
                                return None;
                            }
                            let [x, y, width, height] = components;
                            Some(json!([id, x, y, width, height]))
                        })
                        .collect(),
                )
            }
            _ => return Ok(None),
        };
        Ok(Some(result))
    }

    /// Handles the same method and returns its Flutter success envelope for
    /// the binary messenger. Return None for an unrelated method.
    pub fn reply(&mut self, bytes: &[u8]) -> Result<Option<Vec<u8>>, FlutterScribbleError> {
        self.handle_call(bytes)?
            .map(|value| JsonMethodCodec::encode_success_envelope(&value).map_err(Into::into))
            .transpose()
    }
}

fn coordinate(value: Option<&Value>) -> Result<f32, FlutterScribbleError> {
    value
        .and_then(Value::as_f64)
        .map(|value| value as f32)
        .filter(|value| value.is_finite())
        .ok_or(FlutterScribbleError::InvalidArguments)
}

#[derive(Debug, thiserror::Error)]
pub enum FlutterScribbleError {
    #[error(transparent)]
    Codec(#[from] JsonMethodCodecError),
    #[error("invalid handwriting geometry or element ID")]
    InvalidArguments,
}
