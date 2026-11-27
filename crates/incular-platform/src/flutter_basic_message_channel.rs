//! Typed request and reply on a Flutter-compatible binary messenger.

use std::{convert::Infallible, marker::PhantomData, str::Utf8Error};

use serde_json::Value;

use crate::{
    FlutterBinaryPayload, FlutterInboundMessenger, FlutterPlatformMessenger,
    flutter_inbound_messenger::encode_reply,
    message_codecs::{
        BinaryCodec, JsonMessageCodec, MessageCodecError, MessageValue, StandardMessageCodec,
        StringCodec,
    },
};

/// Encodes values in both directions of a basic message channel. Null
/// payloads and buffers containing zero bytes remain distinguishable.
pub trait FlutterMessageCodec {
    type Message;
    type Error;

    fn encode(message: Self::Message) -> Result<FlutterBinaryPayload, Self::Error>;
    fn decode(payload: FlutterBinaryPayload) -> Result<Self::Message, Self::Error>;
}

impl FlutterMessageCodec for BinaryCodec {
    type Message = FlutterBinaryPayload;
    type Error = Infallible;

    fn encode(message: Self::Message) -> Result<FlutterBinaryPayload, Self::Error> {
        Ok(message)
    }

    fn decode(payload: FlutterBinaryPayload) -> Result<Self::Message, Self::Error> {
        Ok(payload)
    }
}

impl FlutterMessageCodec for StringCodec {
    type Message = Option<String>;
    type Error = Utf8Error;

    fn encode(message: Self::Message) -> Result<FlutterBinaryPayload, Self::Error> {
        Ok(message.map(String::into_bytes))
    }

    fn decode(payload: FlutterBinaryPayload) -> Result<Self::Message, Self::Error> {
        Ok(Self::decode_message(payload.as_deref())?.map(str::to_owned))
    }
}

impl FlutterMessageCodec for JsonMessageCodec {
    type Message = Value;
    type Error = serde_json::Error;

    fn encode(message: Self::Message) -> Result<FlutterBinaryPayload, Self::Error> {
        Self::encode_message(&message)
    }

    fn decode(payload: FlutterBinaryPayload) -> Result<Self::Message, Self::Error> {
        Self::decode_message(payload.as_deref())
    }
}

impl FlutterMessageCodec for StandardMessageCodec {
    type Message = MessageValue;
    type Error = MessageCodecError;

    fn encode(message: Self::Message) -> Result<FlutterBinaryPayload, Self::Error> {
        Self::encode_message(&message)
    }

    fn decode(payload: FlutterBinaryPayload) -> Result<Self::Message, Self::Error> {
        Self::decode_message(payload.as_deref())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum FlutterBasicMessageError<CodecError, TransportError> {
    #[error("while processing a basic message: {0}")]
    Codec(CodecError),
    #[error("while sending a basic message: {0}")]
    Transport(TransportError),
}

/// Reply to a decoded basic message. It can be completed in a later local task.
pub type FlutterBasicMessageReply<C> = Box<
    dyn FnOnce(
        <C as FlutterMessageCodec>::Message,
    ) -> Result<(), <C as FlutterMessageCodec>::Error>,
>;

/// A typed channel where the sender and receiver use the same message codec.
/// Unlike a method channel, there is no method name or envelope.
#[derive(Clone, Copy, Debug)]
pub struct FlutterBasicMessageChannel<C> {
    name: &'static str,
    codec: PhantomData<fn() -> C>,
}

impl<C: FlutterMessageCodec + 'static> FlutterBasicMessageChannel<C> {
    pub const fn new(name: &'static str) -> Self {
        Self {
            name,
            codec: PhantomData,
        }
    }

    pub const fn name(&self) -> &'static str {
        self.name
    }

    pub async fn send<M: FlutterPlatformMessenger>(
        &self,
        messenger: &mut M,
        message: C::Message,
    ) -> Result<C::Message, FlutterBasicMessageError<C::Error, M::Error>> {
        let payload = C::encode(message).map_err(FlutterBasicMessageError::Codec)?;
        let reply = messenger
            .send_platform_message(self.name, payload)
            .await
            .map_err(FlutterBasicMessageError::Transport)?;
        C::decode(reply).map_err(FlutterBasicMessageError::Codec)
    }

    pub fn set_message_handler(
        &self,
        inbound: &FlutterInboundMessenger,
        handler: impl Fn(C::Message, FlutterBasicMessageReply<C>) + 'static,
    ) {
        inbound.set_message_handler(self.name, move |payload, reply| match C::decode(payload) {
            Ok(message) => handler(message, Box::new(encode_reply(reply, C::encode))),
            Err(_) => reply(None),
        });
    }

    pub fn clear_message_handler(&self, inbound: &FlutterInboundMessenger) {
        inbound.clear_message_handler(self.name);
    }
}
