//! Opt-in Flutter application-exit protocol. The embedder owns shutdown:
//! this adapter only sends requests and answers queries.

use std::{
    any::Any,
    cell::RefCell,
    collections::BTreeMap,
    future::{Future, poll_fn},
    panic::{AssertUnwindSafe, catch_unwind},
    pin::Pin,
    rc::{Rc, Weak},
    task::Poll,
};

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

type ExitFuture = Pin<Box<dyn Future<Output = AppExitResponse>>>;
type ExitCallback = Rc<dyn Fn() -> ExitFuture>;

#[derive(Default)]
struct ExitRegistry {
    listeners: BTreeMap<u64, ExitCallback>,
    next_id: u64,
}

/// The exit observers owned by one application binding. Clones allow an
/// observer to be disposed or added while an asynchronous decision is pending.
#[derive(Clone, Default)]
pub struct AppExitObservers(Rc<RefCell<ExitRegistry>>);

/// A subscription remains active until disposed or dropped.
pub struct AppExitSubscription {
    scope: Weak<RefCell<ExitRegistry>>,
    id: u64,
}

impl AppExitSubscription {
    /// Removes the observer. Returns whether it was still registered.
    pub fn dispose(&self) -> bool {
        let callback = self
            .scope
            .upgrade()
            .and_then(|scope| scope.borrow_mut().listeners.remove(&self.id));
        // Releasing captures can itself register or unregister callbacks.
        callback.is_some()
    }

    #[must_use]
    pub const fn id(&self) -> u64 {
        self.id
    }
}

impl Drop for AppExitSubscription {
    fn drop(&mut self) {
        self.dispose();
    }
}

/// A failed exit observer; equivalent to Flutter reporting a throwing
/// `WidgetsBindingObserver.didRequestAppExit` and continuing the query.
pub struct AppExitObserverFailure {
    pub observer: u64,
    pub payload: Box<dyn Any + Send>,
}

impl AppExitObservers {
    /// Registers an asynchronous decision in notification order.
    #[must_use]
    pub fn add_observer<F>(&self, callback: impl Fn() -> F + 'static) -> AppExitSubscription
    where
        F: Future<Output = AppExitResponse> + 'static,
    {
        let mut scope = self.0.borrow_mut();
        let id = scope.next_id;
        scope.next_id = id.checked_add(1).expect("app exit observer ids exhausted");
        scope
            .listeners
            .insert(id, Rc::new(move || Box::pin(callback())));
        AppExitSubscription {
            scope: Rc::downgrade(&self.0),
            id,
        }
    }

    /// Asks each observer present when the request starts, in order. An
    /// observer removed before its turn is skipped. A cancellation determines
    /// the final result but does not prevent later observers from being asked.
    pub async fn request_exit(
        &self,
        mut report_failure: impl FnMut(AppExitObserverFailure),
    ) -> AppExitResponse {
        let snapshot = self
            .0
            .borrow()
            .listeners
            .keys()
            .copied()
            .collect::<Vec<_>>();
        let mut decision = AppExitResponse::Exit;
        for observer in snapshot {
            let callback = self.0.borrow().listeners.get(&observer).cloned();
            let Some(callback) = callback else {
                continue;
            };
            match invoke_exit_observer(callback).await {
                Ok(AppExitResponse::Cancel) => decision = AppExitResponse::Cancel,
                Ok(AppExitResponse::Exit) => {}
                Err(payload) => report_failure(AppExitObserverFailure { observer, payload }),
            }
        }
        decision
    }
}

async fn invoke_exit_observer(
    callback: ExitCallback,
) -> Result<AppExitResponse, Box<dyn Any + Send>> {
    let mut future = catch_unwind(AssertUnwindSafe(|| callback()))?;
    poll_fn(
        |context| match catch_unwind(AssertUnwindSafe(|| future.as_mut().poll(context))) {
            Ok(poll) => poll.map(Ok),
            Err(payload) => Poll::Ready(Err(payload)),
        },
    )
    .await
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
