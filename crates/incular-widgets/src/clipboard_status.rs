//! Observable clipboard presence for editable toolbars.

use std::{cell::Cell, future::Future, pin::Pin, rc::Rc};

use incular_core::{ValueNotifier, reactivity::Subscription};
use incular_platform::{AppLifecycleState, FlutterClipboardCodec, FlutterPlatformMessenger};

/// Whether the host has pasteable plain text, or its state is not known yet.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ClipboardStatus {
    Pasteable,
    #[default]
    Unknown,
    NotPasteable,
}

/// An async clipboard refresh, to be scheduled on the UI owner's executor.
pub type ClipboardStatusTask = Pin<Box<dyn Future<Output = ()>>>;
type Query = Pin<Box<dyn Future<Output = Result<bool, String>>>>;

struct Inner {
    value: ValueNotifier<ClipboardStatus>,
    disposed: Cell<bool>,
    query: Box<dyn Fn() -> Query>,
    schedule: Box<dyn Fn(ClipboardStatusTask)>,
    report_error: Box<dyn Fn(&str)>,
}

/// Flutter's clipboard presence notifier. It queries only for whether text
/// exists, not for the clipboard contents.
#[derive(Clone)]
pub struct ClipboardStatusNotifier(Rc<Inner>);

impl ClipboardStatusNotifier {
    /// Uses the host's `Clipboard.hasStrings` method. A failed query is
    /// reported and returns the notifier to Unknown without propagating an
    /// error to the toolbar. The executor and messenger belong to the host.
    #[must_use]
    pub fn new<M>(
        initial: ClipboardStatus,
        messenger: M,
        schedule: impl Fn(ClipboardStatusTask) + 'static,
        report_error: impl Fn(&str) + 'static,
    ) -> Self
    where
        M: FlutterPlatformMessenger + Clone + 'static,
        M::Error: std::fmt::Display,
    {
        let query = move || -> Query {
            let mut messenger = messenger.clone();
            Box::pin(async move {
                let payload =
                    FlutterClipboardCodec::has_strings_request().map_err(|e| e.to_string())?;
                let reply = messenger
                    .send_platform_message(FlutterClipboardCodec::CHANNEL, Some(payload))
                    .await
                    .map_err(|error| error.to_string())?
                    .ok_or_else(|| {
                        "Clipboard.hasStrings was not implemented by the host".to_owned()
                    })?;
                FlutterClipboardCodec::decode_has_strings_reply(Some(&reply))
                    .map_err(|error| error.to_string())
            })
        };
        Self(Rc::new(Inner {
            value: ValueNotifier::new(initial),
            disposed: Cell::new(false),
            query: Box::new(query),
            schedule: Box::new(schedule),
            report_error: Box::new(report_error),
        }))
    }

    #[must_use]
    pub fn value(&self) -> ClipboardStatus {
        self.0.value.value()
    }

    /// Refreshes on an unknown value when a listener is added. Keep the
    /// subscription alive to continue observing changes.
    pub fn add_listener(&self, listener: impl Fn() + 'static) -> Subscription {
        assert!(
            !self.0.disposed.get(),
            "clipboard status notifier was disposed"
        );
        let subscription = self.0.value.add_listener(listener);
        if self.value() == ClipboardStatus::Unknown {
            self.schedule_update();
        }
        subscription
    }

    pub async fn update(&self) {
        if self.0.disposed.get() {
            return;
        }
        let status = match (self.0.query)().await {
            Ok(true) => ClipboardStatus::Pasteable,
            Ok(false) => ClipboardStatus::NotPasteable,
            Err(error) => {
                (self.0.report_error)(&error);
                ClipboardStatus::Unknown
            }
        };
        if !self.0.disposed.get() {
            self.0.value.set_value(status);
        }
    }

    /// Route the app lifecycle observer here. As in Flutter, a resume only
    /// matters while there is a listener; dropping the last subscription
    /// stops refreshing on later resumes.
    pub fn did_change_app_lifecycle_state(&self, state: AppLifecycleState) {
        if state == AppLifecycleState::Resumed
            && !self.0.disposed.get()
            && self.0.value.has_listeners()
        {
            self.schedule_update();
        }
    }

    /// Stops refreshes and makes any in-flight results inert.
    pub fn dispose(&self) {
        self.0.disposed.set(true);
    }

    fn schedule_update(&self) {
        // A UI owner may discard its scheduled work on shutdown. The queued
        // work must not keep an otherwise dropped notifier alive.
        let weak = Rc::downgrade(&self.0);
        (self.0.schedule)(Box::pin(async move {
            if let Some(inner) = weak.upgrade() {
                Self(inner).update().await;
            }
        }));
    }
}
