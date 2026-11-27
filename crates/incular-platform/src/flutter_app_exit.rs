//! Opt-in Flutter application-exit protocol. The embedder owns shutdown:
//! this adapter only sends requests and answers queries.

use std::future::Future;

use serde_json::{Value, json};

use crate::{
    FlutterJsonMethodChannel, FlutterJsonMethodResult, FlutterPlatformChannelError,
    FlutterPlatformMessenger, message_codecs::MethodCall,
};

/// Controls whether the embedder should ask the app before leaving.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AppExitType {
    Cancelable,
    Required,
}

impl AppExitType {
    const fn name(self) -> &'static str {
        match self {
            Self::Cancelable => "cancelable",
            Self::Required => "required",
        }
    }
}

/// Result of an app-exit request. A host query defaults to permitting exit.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum AppExitResponse {
    #[default]
    Exit,
    Cancel,
}

impl AppExitResponse {
    const fn name(self) -> &'static str {
        match self {
            Self::Exit => "exit",
            Self::Cancel => "cancel",
        }
    }
}

/// Flutter's application-exit calls on the optional platform channel.
///
/// This does not call process exit or install a channel handler over other
/// platform features. An embedder can use [Self::route_request] as one arm
/// of its existing method handler.
pub struct FlutterAppExit;

impl FlutterAppExit {
    pub const CHANNEL: FlutterJsonMethodChannel =
        FlutterJsonMethodChannel::optional("flutter/platform");
    pub const REQUEST_METHOD: &str = "System.requestAppExit";

    /// Requests an orderly exit. An absent exit code is sent as zero, and
    /// a missing or null platform result is interpreted as cancellation.
    pub async fn exit_application<M: FlutterPlatformMessenger>(
        messenger: &mut M,
        kind: AppExitType,
        exit_code: Option<i32>,
    ) -> Result<AppExitResponse, FlutterPlatformChannelError<M::Error>> {
        let response = Self::CHANNEL
            .invoke_map::<Value, _>(
                messenger,
                "System.exitApplication",
                json!({ "type": kind.name(), "exitCode": exit_code.unwrap_or(0) }),
            )
            .await?;
        Ok(match response {
            None => AppExitResponse::Cancel,
            Some(map) if map.get("response").and_then(Value::as_str) == Some("cancel") => {
                AppExitResponse::Cancel
            }
            Some(_) => AppExitResponse::Exit,
        })
    }

    /// Returns a response to an engine exit query, or leaves unrelated
    /// methods unhandled so other platform services can process them.
    /// For the default Flutter binding behavior, supply
    /// `|| async { AppExitResponse::default() }`.
    pub async fn route_request<F: Future<Output = AppExitResponse>>(
        call: &MethodCall<Value>,
        decide: impl FnOnce() -> F,
    ) -> Option<FlutterJsonMethodResult> {
        if call.method != Self::REQUEST_METHOD {
            return None;
        }
        Some(FlutterJsonMethodResult::Success(
            json!({ "response": decide().await.name() }),
        ))
    }
}
