//! Typed Flutter accessibility events for embedders with a Flutter-protocol
//! transport. Native accessibility backends can keep their own delivery APIs.

use std::future::Future;

use incular_config::TextDirection;

use crate::message_codecs::{MessageCodecError, MessageValue, StandardMessageCodec};

/// Urgency requested by an accessibility announcement.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Assertiveness {
    /// Announce when assistive technology is idle.
    #[default]
    Polite,
    /// Request interruption for a time-sensitive announcement.
    Assertive,
}

/// Text announced on a specific view.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SemanticsAnnouncement {
    pub view_id: i64,
    pub message: String,
    pub text_direction: TextDirection,
    pub assertiveness: Assertiveness,
}

impl SemanticsAnnouncement {
    /// Creates a polite announcement, Flutter's default.
    #[must_use]
    pub fn new(view_id: i64, message: impl Into<String>, direction: TextDirection) -> Self {
        Self {
            view_id,
            message: message.into(),
            text_direction: direction,
            assertiveness: Assertiveness::Polite,
        }
    }

    #[must_use]
    pub const fn assertiveness(mut self, assertiveness: Assertiveness) -> Self {
        self.assertiveness = assertiveness;
        self
    }
}

/// A notification delivered to Flutter-compatible accessibility transports.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SemanticsEvent {
    Announce(SemanticsAnnouncement),
    Tooltip(String),
    Tap,
    LongPress,
    Focus,
}

impl SemanticsEvent {
    /// Encodes the standard accessibility envelope. The associated node is
    /// optional and goes beside the event data, not inside it.
    pub fn encode(self, node_id: Option<i64>) -> Result<Vec<u8>, MessageCodecError> {
        use MessageValue::{Int, Map, String};
        let kv = |name: &str, value| (String(name.into()), value);
        let (kind, fields) = match self {
            Self::Announce(announcement) => {
                let mut fields = vec![
                    kv("viewId", Int(announcement.view_id)),
                    kv("message", String(announcement.message)),
                    kv(
                        "textDirection",
                        Int(match announcement.text_direction {
                            TextDirection::Rtl => 0,
                            TextDirection::Ltr => 1,
                        }),
                    ),
                ];
                if announcement.assertiveness == Assertiveness::Assertive {
                    fields.push(kv("assertiveness", Int(1)));
                }
                ("announce", fields)
            }
            Self::Tooltip(message) => ("tooltip", vec![kv("message", String(message))]),
            Self::Tap => ("tap", Vec::new()),
            Self::LongPress => ("longPress", Vec::new()),
            Self::Focus => ("focus", Vec::new()),
        };
        let mut envelope = vec![kv("type", String(kind.into())), kv("data", Map(fields))];
        if let Some(node_id) = node_id {
            envelope.push(kv("nodeId", Int(node_id)));
        }
        Ok(StandardMessageCodec::encode_message(&Map(envelope))?
            .expect("a non-null accessibility map always encodes"))
    }
}

/// Sends encoded accessibility events on behalf of an embedder.
pub trait AccessibilityMessenger {
    type Error;

    fn send_accessibility_message(
        &mut self,
        channel: &'static str,
        payload: Vec<u8>,
    ) -> impl Future<Output = Result<(), Self::Error>>;
}

/// The codec or platform transport failed while sending a semantics message.
#[derive(Debug, thiserror::Error)]
pub enum SemanticsMessageError<E> {
    #[error(transparent)]
    Encode(#[from] MessageCodecError),
    #[error("while sending accessibility event: {0}")]
    Send(E),
}

/// Async failure reported by an attached semantic node. The original
/// transport error is preserved alongside its framework context.
pub struct SemanticsNodeErrorReport<'a, E> {
    pub node_id: i64,
    pub error: &'a SemanticsMessageError<E>,
    pub library: &'static str,
    pub context: &'static str,
}

type NodeErrorReporter<E> = Box<dyn for<'a> FnMut(SemanticsNodeErrorReport<'a, E>)>;

/// Flutter's `SemanticsService` for embedders that own an accessibility
/// channel. It waits for transport delivery; it does not speak the text itself.
pub struct SemanticsService<M: AccessibilityMessenger> {
    messenger: M,
    node_error_reporter: Option<NodeErrorReporter<M::Error>>,
}

impl<M: AccessibilityMessenger> SemanticsService<M> {
    pub const CHANNEL: &str = "flutter/accessibility";

    #[must_use]
    pub const fn new(messenger: M) -> Self {
        Self {
            messenger,
            node_error_reporter: None,
        }
    }

    /// Registers the embedder's framework error handler for node events.
    /// The typed Result is still returned to Rust callers. Announcements
    /// made directly through the service report errors to their caller only.
    pub fn set_node_error_reporter(
        &mut self,
        reporter: impl for<'a> FnMut(SemanticsNodeErrorReport<'a, M::Error>) + 'static,
    ) {
        self.node_error_reporter = Some(Box::new(reporter));
    }

    pub fn clear_node_error_reporter(&mut self) {
        self.node_error_reporter = None;
    }

    /// Sends one view-targeted announcement without a node ID.
    pub async fn send_announcement(
        &mut self,
        announcement: SemanticsAnnouncement,
    ) -> Result<(), SemanticsMessageError<M::Error>> {
        self.send(None, SemanticsEvent::Announce(announcement))
            .await
    }

    /// Sends an event for an attached node. Detached nodes are ignored
    /// without encoding or invoking the transport.
    pub async fn send_node_event(
        &mut self,
        attached_node_id: Option<i64>,
        event: SemanticsEvent,
    ) -> Result<bool, SemanticsMessageError<M::Error>> {
        let Some(node_id) = attached_node_id else {
            return Ok(false);
        };
        let result = self.send(Some(node_id), event).await;
        if let Err(error) = &result
            && let Some(reporter) = &mut self.node_error_reporter
        {
            reporter(SemanticsNodeErrorReport {
                node_id,
                error,
                library: "semantics library",
                context: "while sending accessibility event",
            });
        }
        result.map(|()| true)
    }

    async fn send(
        &mut self,
        node_id: Option<i64>,
        event: SemanticsEvent,
    ) -> Result<(), SemanticsMessageError<M::Error>> {
        let bytes = event.encode(node_id)?;
        self.messenger
            .send_accessibility_message(Self::CHANNEL, bytes)
            .await
            .map_err(SemanticsMessageError::Send)
    }
}
