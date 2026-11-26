//! Native filesystem notifications for preview source and manifest changes.

use std::{
    collections::{BTreeMap, VecDeque},
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use notify::{
    Config, ErrorKind, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher,
    event::{CreateKind, ModifyKind, RemoveKind},
};
use tokio::sync::{Notify, mpsc};

use crate::preview_changes::{PreviewChange, PreviewChangeRouter};

/// An accepted filesystem notification or a request for a full refresh.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PreviewWatchUpdate {
    /// A registered source or manifest changed.
    Change(PreviewChange),
    /// The scope changed or the backend could not retain all events.
    RescanRequired,
}

/// A native watch failed or its configured source scope could not be read.
#[derive(Debug, thiserror::Error)]
pub enum PreviewWatchError {
    /// Couldn't resolve the configured watch directories.
    #[error("could not read preview watch scope: {0}")]
    Scope(#[from] std::io::Error),
    /// The selected operating-system watcher failed.
    #[error("preview filesystem watcher failed: {0}")]
    Backend(#[from] notify::Error),
}

#[derive(Default)]
struct Overflow {
    dropped: AtomicBool,
    signal: Notify,
}

/// Watches only registered preview source directories and manifest parents.
/// There is no application polling loop. Dropping it releases the backend.
pub struct PreviewWatcher {
    router: PreviewChangeRouter,
    watcher: RecommendedWatcher,
    installed: BTreeMap<PathBuf, bool>,
    events: mpsc::Receiver<notify::Result<Event>>,
    overflow: Arc<Overflow>,
    pending: VecDeque<PreviewChange>,
}

impl PreviewWatcher {
    /// Starts a bounded native watcher for this router's current workspace.
    pub fn start(router: PreviewChangeRouter) -> Result<Self, PreviewWatchError> {
        let (sender, events) = mpsc::channel(128);
        let overflow = Arc::new(Overflow::default());
        let signal = overflow.clone();
        let mut stream = Self {
            router,
            watcher: RecommendedWatcher::new(
                move |event| {
                    if matches!(
                        sender.try_send(event),
                        Err(mpsc::error::TrySendError::Full(_))
                    ) {
                        signal.dropped.store(true, Ordering::Release);
                        signal.signal.notify_one();
                    }
                },
                Config::default().with_follow_symlinks(false),
            )?,
            installed: BTreeMap::new(),
            events,
            overflow,
            pending: VecDeque::new(),
        };
        stream.refresh_scope()?;
        Ok(stream)
    }

    /// Waits for the next relevant change. Irrelevant accesses and metadata
    /// events do not trigger reanalysis. A rescan notice must be handled before
    /// any caller treats the pending event stream as a complete history.
    pub async fn next_update(&mut self) -> Result<Option<PreviewWatchUpdate>, PreviewWatchError> {
        loop {
            if self.overflow.dropped.swap(false, Ordering::AcqRel) {
                self.refresh_scope()?;
                return Ok(Some(PreviewWatchUpdate::RescanRequired));
            }
            if let Some(change) = self.pending.pop_front() {
                return Ok(Some(PreviewWatchUpdate::Change(change)));
            }
            let event = tokio::select! {
                biased;
                _ = self.overflow.signal.notified() => continue,
                event = self.events.recv() => event,
            };
            let Some(event) = event else { return Ok(None) };
            let event = event?;
            if event.need_rescan() {
                self.refresh_scope()?;
                return Ok(Some(PreviewWatchUpdate::RescanRequired));
            }
            if matches!(
                event.kind,
                EventKind::Create(CreateKind::Folder | CreateKind::Any)
                    | EventKind::Remove(RemoveKind::Folder | RemoveKind::Any)
                    | EventKind::Modify(ModifyKind::Name(_))
            ) && self.refresh_scope()?
            {
                // A newly created source directory can already contain files
                // by the time its recursive watch is installed.
                return Ok(Some(PreviewWatchUpdate::RescanRequired));
            }
            if matches!(
                event.kind,
                EventKind::Any
                    | EventKind::Create(_)
                    | EventKind::Remove(_)
                    | EventKind::Modify(
                        ModifyKind::Any | ModifyKind::Data(_) | ModifyKind::Name(_)
                    )
            ) {
                for path in event.paths {
                    if !path.is_dir()
                        && let Some(change) = self.router.route(path)
                        && !self.pending.contains(&change)
                    {
                        self.pending.push_back(change);
                    }
                }
            }
        }
    }

    fn refresh_scope(&mut self) -> Result<bool, PreviewWatchError> {
        let next = self.router.watch_plan()?;
        if self.installed == next {
            return Ok(false);
        }
        for (path, recursive) in &next {
            if self.installed.get(path) != Some(recursive) {
                let mode = if *recursive {
                    RecursiveMode::Recursive
                } else {
                    RecursiveMode::NonRecursive
                };
                self.watcher.watch(path, mode)?;
                self.installed.insert(path.clone(), *recursive);
            }
        }
        let stale: Vec<_> = self
            .installed
            .keys()
            .filter(|path| !next.contains_key(*path))
            .cloned()
            .collect();
        for path in stale {
            if let Err(error) = self.watcher.unwatch(&path)
                && !matches!(error.kind, ErrorKind::WatchNotFound)
            {
                return Err(error.into());
            }
            self.installed.remove(&path);
        }
        Ok(true)
    }
}
