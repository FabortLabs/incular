//! Copy-to-clipboard for buttons that have no access to the runtime.
//!
//! Button callbacks run without a `&mut Runtime`, so a copy request goes
//! through a channel to a UI-thread task that owns the clipboard write and
//! re-arms itself after each one.

use super::shared::refresh;
use crate::inspector::Shared;
use incular::prelude::*;
use std::sync::Arc;
use tokio::sync::{
    Mutex,
    mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel},
};

/// Longest text a single copy places on the clipboard.
const MAX_COPY_BYTES: usize = 1024 * 1024;

#[derive(Clone)]
pub(crate) struct Clipboard {
    sender: UnboundedSender<String>,
    receiver: Arc<Mutex<UnboundedReceiver<String>>>,
    shared: Shared,
    tick: Signal<u64>,
}

impl Clipboard {
    pub(crate) fn new(shared: Shared, tick: Signal<u64>) -> Self {
        let (sender, receiver) = unbounded_channel();
        Self {
            sender,
            receiver: Arc::new(Mutex::new(receiver)),
            shared,
            tick,
        }
    }

    /// Places `text` on the clipboard and confirms `what` in the status bar.
    pub(crate) fn copy(&self, mut text: String, what: &str) {
        if text.len() > MAX_COPY_BYTES {
            let mut end = MAX_COPY_BYTES;
            while !text.is_char_boundary(end) {
                end -= 1;
            }
            text.truncate(end);
        }
        let sent = self.sender.send(text).is_ok();
        if let Ok(mut state) = self.shared.lock() {
            state.notice = Some(if sent {
                format!("Copied {what} to the clipboard")
            } else {
                "The clipboard is unavailable".to_owned()
            });
        }
        refresh(&self.tick);
    }

    /// Starts the UI-thread writer; call once from the root build.
    pub(crate) fn start(&self, cx: &BuildContext) {
        let scope = cx.task_scope();
        let receiver = Arc::clone(&self.receiver);
        let next = Arc::clone(&self.receiver);
        cx.spawn_into(
            async move { receiver.lock().await.recv().await },
            move |text, runtime| {
                if let Ok(Some(text)) = text {
                    runtime.set_clipboard_text(text);
                    arm(runtime.spawner(), scope, next);
                }
            },
        );
    }
}

fn arm(
    spawner: incular::runtime::RuntimeSpawner,
    scope: TaskScope,
    receiver: Arc<Mutex<UnboundedReceiver<String>>>,
) {
    if scope.is_cancelled() {
        return;
    }
    let next = Arc::clone(&receiver);
    let next_scope = scope.clone();
    spawner.spawn_into_in(
        &scope,
        async move { receiver.lock().await.recv().await },
        move |text, runtime| {
            if let Ok(Some(text)) = text {
                runtime.set_clipboard_text(text);
                arm(runtime.spawner(), next_scope, next);
            }
        },
    );
}
