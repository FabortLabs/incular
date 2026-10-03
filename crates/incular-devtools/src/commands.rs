//! UI-thread command envelopes.
//!
//! A request owns its completion channel for its entire lifetime. The UI never
//! routes replies by request id, so cancellation or reconnect cannot deliver a
//! result to another session.

use incular_devtools_protocol::{ErrorCode, RequestMethod, ResponsePayload};
use std::sync::{Arc, atomic::AtomicUsize};

#[derive(Debug)]
pub(crate) struct CompletedResponse {
    pub(crate) result: Result<ResponsePayload, ErrorCode>,
    _permit: Option<crate::BytePermit>,
}

/// A one-use, non-blocking completion capability for one admitted command.
#[derive(Debug)]
pub struct CommandCompletion {
    sender: Option<tokio::sync::oneshot::Sender<CompletedResponse>>,
    response_payload_bytes: Arc<AtomicUsize>,
    request_id: u64,
    request_kind: &'static str,
}

impl CommandCompletion {
    pub(crate) fn new(
        sender: tokio::sync::oneshot::Sender<CompletedResponse>,
        response_payload_bytes: Arc<AtomicUsize>,
        request_id: u64,
        request_kind: &'static str,
    ) -> Self {
        Self {
            sender: Some(sender),
            response_payload_bytes,
            request_id,
            request_kind,
        }
    }

    /// Returns true once the originating session has abandoned this command.
    #[must_use]
    pub fn is_abandoned(&self) -> bool {
        self.sender
            .as_ref()
            .is_none_or(tokio::sync::oneshot::Sender::is_closed)
    }

    /// Completes the command. Delivery never waits for the network peer.
    pub fn complete(mut self, result: Result<ResponsePayload, ErrorCode>) {
        if let Some(sender) = self.sender.take() {
            let (result, permit) = match serde_json::to_vec(&result) {
                Ok(payload) => match crate::BytePermit::try_acquire(
                    &self.response_payload_bytes,
                    payload.len(),
                    crate::RESPONSE_PAYLOAD_BUDGET,
                ) {
                    Some(permit) => (result, Some(permit)),
                    None => {
                        eprintln!(
                            "DevTools target rejected request {} ({}): response payload budget exceeded",
                            self.request_id, self.request_kind
                        );
                        (Err(ErrorCode::InternalError), None)
                    }
                },
                Err(error) => {
                    eprintln!(
                        "DevTools target response serialization failed for request {} ({}): {error}",
                        self.request_id, self.request_kind
                    );
                    (Err(ErrorCode::InternalError), None)
                }
            };
            let _ = sender.send(CompletedResponse {
                result,
                _permit: permit,
            });
        }
    }
}

/// One request routed to the UI thread with its session-owned completion.
#[derive(Debug)]
pub struct UiCommand {
    /// Protocol request to execute on the application's UI thread.
    pub body: RequestMethod,
    /// Session-owned capability used to complete this request exactly once.
    pub completion: CommandCompletion,
    pub(crate) _request_permit: crate::BytePermit,
}
