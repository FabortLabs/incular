//! Text processing requests understood by Flutter-compatible platform hosts.

use std::future::Future;

use crate::{
    FlutterPlatformMessenger,
    message_codecs::{MessageValue, MethodCall, MethodCodecError, StandardMethodCodec},
};

/// An available text processing action and its localized label.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ProcessTextAction {
    pub id: String,
    pub label: String,
}

pub trait ProcessTextService {
    type Error;
    fn query_text_actions(
        &mut self,
    ) -> impl Future<Output = Result<Vec<ProcessTextAction>, Self::Error>>;
    fn process_text_action(
        &mut self,
        id: &str,
        text: &str,
        read_only: bool,
    ) -> impl Future<Output = Result<Option<String>, Self::Error>>;
}

#[derive(Debug, thiserror::Error)]
pub enum ProcessTextError<E> {
    #[error(transparent)]
    Codec(#[from] MethodCodecError),
    #[error("while sending a process-text platform command: {0}")]
    Transport(E),
    #[error("process-text action identifiers and labels must be strings")]
    InvalidAction,
    #[error("a process-text action must return a string or null")]
    InvalidProcessedText,
}

/// Default StandardMethodCodec service. The embedder owns any native
/// application discovery or invocation.
pub struct DefaultProcessTextService<M> {
    messenger: M,
}

impl<M: FlutterPlatformMessenger> DefaultProcessTextService<M> {
    pub const CHANNEL: &str = "flutter/processtext";

    #[must_use]
    pub const fn new(messenger: M) -> Self {
        Self { messenger }
    }

    async fn invoke(
        &mut self,
        method: &str,
        arguments: MessageValue,
    ) -> Result<MessageValue, ProcessTextError<M::Error>> {
        let payload = StandardMethodCodec::encode_method_call(&MethodCall {
            method: method.to_owned(),
            arguments,
        })?;
        let reply = self
            .messenger
            .send_platform_message(Self::CHANNEL, Some(payload))
            .await
            .map_err(ProcessTextError::Transport)?;
        match reply {
            Some(bytes) => Ok(StandardMethodCodec::decode_envelope(&bytes)?),
            None => Ok(MessageValue::Null),
        }
    }
}

impl<M: FlutterPlatformMessenger> ProcessTextService for DefaultProcessTextService<M> {
    type Error = ProcessTextError<M::Error>;

    async fn query_text_actions(&mut self) -> Result<Vec<ProcessTextAction>, Self::Error> {
        let Ok(MessageValue::Map(actions)) = self
            .invoke("ProcessText.queryTextActions", MessageValue::Null)
            .await
        else {
            // Flutter treats a missing, failed, null or non-map query as
            // no available actions. A malformed individual entry is an error.
            return Ok(Vec::new());
        };
        actions
            .into_iter()
            .map(|(id, label)| match (id, label) {
                (MessageValue::String(id), MessageValue::String(label)) => {
                    Ok(ProcessTextAction { id, label })
                }
                _ => Err(ProcessTextError::InvalidAction),
            })
            .collect()
    }

    async fn process_text_action(
        &mut self,
        id: &str,
        text: &str,
        read_only: bool,
    ) -> Result<Option<String>, Self::Error> {
        let arguments = MessageValue::List(vec![
            MessageValue::String(id.to_owned()),
            MessageValue::String(text.to_owned()),
            MessageValue::Bool(read_only),
        ]);
        match self
            .invoke("ProcessText.processTextAction", arguments)
            .await?
        {
            MessageValue::String(value) => Ok(Some(value)),
            MessageValue::Null => Ok(None),
            _ => Err(ProcessTextError::InvalidProcessedText),
        }
    }
}
