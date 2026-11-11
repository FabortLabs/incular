//! Turns model notifications into the narrowest UI rebuild.
//!
//! The transport wakes the UI for every message, and a connected target
//! streams a frame record for every frame it presents (often 60 to 240 a
//! second). Rebuilding the workbench for each would keep DevTools busy even on
//! a page that shows no frame data. The pump reads the model's change
//! [`Revisions`] and, rate limited:
//!
//! - content changes bump `tick`, rebuilding the workbench (at most ~30 Hz);
//! - telemetry bumps `telemetry`, rebuilding only a page that charts frames
//!   (Overview, Performance; at most 20 Hz), or else only the header pill
//!   (4 Hz);
//! - a new live FPS reading bumps `header`, rebuilding only the pill.

use super::ToolView;
use super::shared::refresh;
use crate::inspector::{Revisions, Shared};
use incular::prelude::*;
use std::{cell::Cell, rc::Rc, sync::Arc, time::Duration};

const CONTENT_INTERVAL: Duration = Duration::from_millis(33);
const TELEMETRY_INTERVAL: Duration = Duration::from_millis(50);
const HEADER_INTERVAL: Duration = Duration::from_millis(250);

#[derive(Clone)]
pub(crate) struct UpdatePump {
    pub(crate) updates: Arc<tokio::sync::Notify>,
    pub(crate) shared: Shared,
    pub(crate) view: Signal<ToolView>,
    pub(crate) tick: Signal<u64>,
    pub(crate) telemetry: Signal<u64>,
    pub(crate) header: Signal<u64>,
    seen: Rc<Cell<Revisions>>,
    last: Rc<Cell<[Option<std::time::Instant>; 3]>>,
}

/// Whether `view` charts streamed frames.
pub(crate) const fn shows_telemetry(view: ToolView) -> bool {
    matches!(view, ToolView::Overview | ToolView::Performance)
}

impl UpdatePump {
    pub(crate) fn new(
        updates: Arc<tokio::sync::Notify>,
        shared: Shared,
        view: Signal<ToolView>,
        tick: Signal<u64>,
        telemetry: Signal<u64>,
        header: Signal<u64>,
    ) -> Self {
        Self {
            updates,
            shared,
            view,
            tick,
            telemetry,
            header,
            seen: Rc::new(Cell::new(Revisions::default())),
            last: Rc::new(Cell::new([None; 3])),
        }
    }

    /// Starts the wait loop; call once from the root build.
    pub(crate) fn start(&self, cx: &BuildContext) {
        let scope = cx.task_scope();
        let pump = self.clone();
        let updates = Arc::clone(&self.updates);
        cx.spawn_into(
            async move { updates.notified().await },
            move |woken, runtime| {
                if woken.is_ok() {
                    let pause = pump.apply();
                    pump.arm(runtime, scope, pause);
                }
            },
        );
    }

    fn arm(&self, runtime: &mut Runtime, scope: TaskScope, pause: Option<Duration>) {
        if scope.is_cancelled() {
            return;
        }
        let pump = self.clone();
        let updates = Arc::clone(&self.updates);
        let next_scope = scope.clone();
        runtime.spawner().spawn_into_in(
            &scope,
            async move {
                match pause {
                    // A change was deferred by the rate limit: re-check after
                    // the pause even if nothing new arrives.
                    Some(pause) => tokio::time::sleep(pause).await,
                    None => updates.notified().await,
                }
            },
            move |woken, runtime| {
                if woken.is_ok() {
                    let pause = pump.apply();
                    pump.arm(runtime, next_scope, pause);
                }
            },
        );
    }

    /// Rebuilds what changed, or returns how long to wait before the rate
    /// limit allows it.
    fn apply(&self) -> Option<Duration> {
        let Ok(now) = self.shared.lock().map(|state| state.revisions) else {
            return None;
        };
        let mut seen = self.seen.get();
        let mut last = self.last.get();
        let mut pause = None::<Duration>;
        let mut defer = |wait: Duration| {
            pause = Some(pause.map_or(wait, |current| current.min(wait)));
        };
        let elapsed = |slot: Option<std::time::Instant>| {
            slot.map_or(Duration::MAX, |instant| instant.elapsed())
        };
        if now.content != seen.content {
            if elapsed(last[0]) >= CONTENT_INTERVAL {
                refresh(&self.tick);
                last[0] = Some(std::time::Instant::now());
                // A full rebuild also redraws the telemetry pages and pill.
                seen = now;
                last[1] = last[0];
                last[2] = last[0];
            } else {
                defer(CONTENT_INTERVAL - elapsed(last[0]));
            }
        }
        if now.telemetry != seen.telemetry {
            let (slot, interval) = if shows_telemetry(self.view.get()) {
                (1, TELEMETRY_INTERVAL)
            } else {
                (2, HEADER_INTERVAL)
            };
            if elapsed(last[slot]) >= interval {
                refresh(if slot == 1 {
                    &self.telemetry
                } else {
                    &self.header
                });
                if slot == 1 {
                    // The header sparkline shares the frame stream.
                    refresh(&self.header);
                }
                last[slot] = Some(std::time::Instant::now());
                seen.telemetry = now.telemetry;
            } else {
                defer(interval - elapsed(last[slot]));
            }
        }
        if now.header != seen.header {
            refresh(&self.header);
            seen.header = now.header;
        }
        self.seen.set(seen);
        self.last.set(last);
        pause
    }
}
