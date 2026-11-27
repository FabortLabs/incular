//! Native filesystem notifications for preview source and manifest changes.

use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    fs, io,
    path::{Path, PathBuf},
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
    source_directories: BTreeSet<PathBuf>,
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
            source_directories: BTreeSet::new(),
            events,
            overflow,
            pending: VecDeque::new(),
        };
        stream.refresh_after_gap()?;
        Ok(stream)
    }

    /// Waits for the next relevant change. Irrelevant accesses and metadata
    /// events do not trigger reanalysis. A rescan notice must be handled before
    /// any caller treats the pending event stream as a complete history.
    pub async fn next_update(&mut self) -> Result<Option<PreviewWatchUpdate>, PreviewWatchError> {
        loop {
            if self.overflow.dropped.swap(false, Ordering::AcqRel) {
                self.refresh_after_gap()?;
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
                self.refresh_after_gap()?;
                return Ok(Some(PreviewWatchUpdate::RescanRequired));
            }
            if matches!(
                event.kind,
                EventKind::Create(CreateKind::Folder | CreateKind::Any)
                    | EventKind::Remove(RemoveKind::Folder | RemoveKind::Any)
                    | EventKind::Modify(ModifyKind::Name(_))
            ) {
                let changed = if self.refresh_scope()? {
                    self.refresh_source_directories()?;
                    true
                } else {
                    let mut changed = false;
                    for path in &event.paths {
                        changed |= self.refresh_source_subtree(path)?;
                    }
                    changed
                };
                if changed {
                    // Recursive backends may report only the directory for
                    // a subtree move, leaving no events for its source files.
                    return Ok(Some(PreviewWatchUpdate::RescanRequired));
                }
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

    fn refresh_after_gap(&mut self) -> Result<(), PreviewWatchError> {
        self.refresh_scope()?;
        self.refresh_source_directories()?;
        Ok(())
    }

    fn refresh_source_directories(&mut self) -> io::Result<()> {
        let stack = self
            .installed
            .iter()
            .filter(|(path, _)| self.router.is_source_directory(path))
            .map(|(path, recursive)| (path.clone(), *recursive))
            .collect();
        self.source_directories = self.collect_source_directories(stack)?;
        Ok(())
    }

    fn refresh_source_subtree(&mut self, path: &Path) -> io::Result<bool> {
        if !self.router.is_source_directory(path) {
            return Ok(false);
        }
        let current = self
            .source_directories
            .range(path.to_owned()..)
            .take_while(|known| known.starts_with(path));
        if !self.source_directories.contains(path)
            && !fs::symlink_metadata(path).is_ok_and(|metadata| metadata.is_dir())
        {
            return Ok(false);
        }
        // Work only on the affected subtree for routine local events.
        let next = self.collect_source_directories(vec![(path.to_owned(), true)])?;
        if current.clone().eq(&next) {
            return Ok(false);
        }
        let stale: Vec<_> = current.cloned().collect();
        for path in stale {
            self.source_directories.remove(&path);
        }
        self.source_directories.extend(next);
        Ok(true)
    }

    fn collect_source_directories(
        &self,
        mut stack: Vec<(PathBuf, bool)>,
    ) -> io::Result<BTreeSet<PathBuf>> {
        let mut next = BTreeSet::new();
        while let Some((path, recursive)) = stack.pop() {
            // A directory can vanish while rename events are still queued.
            let metadata = match fs::symlink_metadata(&path) {
                Ok(metadata) => metadata,
                Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
                Err(error) => return Err(error),
            };
            if !metadata.is_dir() {
                continue;
            }
            if recursive {
                let entries = match fs::read_dir(&path) {
                    Ok(entries) => entries,
                    Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
                    Err(error) => return Err(error),
                };
                for entry in entries {
                    let entry = entry?;
                    match entry.file_type() {
                        Ok(kind) if kind.is_dir() => {
                            let child = entry.path();
                            if self.router.is_source_directory(&child) {
                                stack.push((child, true));
                            }
                        }
                        Ok(_) => {}
                        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                        Err(error) => return Err(error),
                    }
                }
            }
            next.insert(path);
        }
        Ok(next)
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
