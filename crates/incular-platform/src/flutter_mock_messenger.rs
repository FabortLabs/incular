//! Test transport for Flutter-compatible binary and JSON method channels.

use std::{collections::HashMap, convert::Infallible, future::Future, pin::Pin, rc::Rc};

use serde_json::Value;

use crate::{
    FlutterBinaryPayload, FlutterJsonMethodResult, FlutterPlatformMessenger,
    message_codecs::{JsonMethodCodec, MethodCall},
};

type Response = Pin<Box<dyn Future<Output = FlutterBinaryPayload>>>;
type Handler = dyn Fn(FlutterBinaryPayload) -> Response;

/// Stable identity of one asynchronous mock. Clones identify the same
/// handler; independently constructed closures are distinct.
#[derive(Clone)]
pub struct FlutterMockHandler(Rc<Handler>);

impl FlutterMockHandler {
    #[must_use]
    pub fn binary<F>(handler: impl Fn(FlutterBinaryPayload) -> F + 'static) -> Self
    where
        F: Future<Output = FlutterBinaryPayload> + 'static,
    {
        Self(Rc::new(move |payload| Box::pin(handler(payload))))
    }

    #[must_use]
    pub fn json_method<F>(handler: impl Fn(MethodCall<Value>) -> F + 'static) -> Self
    where
        F: Future<Output = FlutterJsonMethodResult> + 'static,
    {
        let handler = Rc::new(handler);
        Self::binary(move |payload| {
            let handler = handler.clone();
            async move {
                let call = JsonMethodCodec::decode_method_call(payload.as_deref()?).ok()?;
                handler(call).await.encode().ok()?
            }
        })
    }
}

/// Returns no reply for an unmocked channel.
#[derive(Default)]
pub struct FlutterMockNullTransport;

impl FlutterPlatformMessenger for FlutterMockNullTransport {
    type Error = Infallible;

    async fn send_platform_message(
        &mut self,
        _channel: &'static str,
        _payload: FlutterBinaryPayload,
    ) -> Result<FlutterBinaryPayload, Self::Error> {
        Ok(None)
    }
}

/// A local mock overlay. Unregistered channels use the supplied transport
/// without rewriting their messages or responses.
pub struct FlutterMockMessenger<M = FlutterMockNullTransport> {
    fallback: M,
    handlers: HashMap<&'static str, FlutterMockHandler>,
}

impl Default for FlutterMockMessenger {
    fn default() -> Self {
        Self::new(FlutterMockNullTransport)
    }
}

impl<M> FlutterMockMessenger<M> {
    #[must_use]
    pub fn new(fallback: M) -> Self {
        Self {
            fallback,
            handlers: HashMap::new(),
        }
    }

    pub fn set_mock_message_handler(
        &mut self,
        channel: &'static str,
        handler: Option<FlutterMockHandler>,
    ) {
        if let Some(handler) = handler {
            self.handlers.insert(channel, handler);
        } else {
            self.handlers.remove(channel);
        }
    }

    /// Checks exact registration identity without sending any message.
    /// `None` matches only when this channel has no mock.
    #[must_use]
    pub fn check_mock_message_handler(
        &self,
        channel: &str,
        expected: Option<&FlutterMockHandler>,
    ) -> bool {
        match (self.handlers.get(channel), expected) {
            (None, None) => true,
            (Some(installed), Some(expected)) => Rc::ptr_eq(&installed.0, &expected.0),
            _ => false,
        }
    }
}

impl<M: FlutterPlatformMessenger> FlutterPlatformMessenger for FlutterMockMessenger<M> {
    type Error = M::Error;

    async fn send_platform_message(
        &mut self,
        channel: &'static str,
        payload: FlutterBinaryPayload,
    ) -> Result<FlutterBinaryPayload, Self::Error> {
        if let Some(handler) = self.handlers.get(channel).cloned() {
            Ok((handler.0)(payload).await)
        } else {
            self.fallback.send_platform_message(channel, payload).await
        }
    }
}
