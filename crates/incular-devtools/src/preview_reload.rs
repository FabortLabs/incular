//! Serialized preview reloads behind the host's watcher and analysis backend.

use std::{
    future::Future,
    num::NonZeroU8,
    path::{Path, PathBuf},
    time::Duration,
};

use tokio::{
    sync::{Mutex, watch},
    time::{Instant, sleep, timeout},
};

use crate::preview_changes::{PreviewChange, PreviewChangeRouter};

/// Operations available to the host for one preview update.
pub trait PreviewReloadHost: Send {
    /// The complete preview snapshot returned by the backend.
    type Previews: Send + Sync;
    /// An error from analysis, lookup, publication, or metadata refresh.
    type Error: std::error::Error + Send + Sync + 'static;

    /// Reports that this host's analysis context was disposed independently
    /// of its watcher or reload coordinator. No lookup or publication is
    /// attempted once the context is gone.
    fn analysis_disposed(&self) -> bool {
        false
    }

    /// Waits for the analysis source already selected by the host.
    fn wait_for_analysis(&mut self) -> impl Future<Output = Result<(), Self::Error>> + Send;
    /// Reads the current previews after a source update. May be retried.
    fn load_previews(
        &mut self,
        source: &Path,
    ) -> impl Future<Output = Result<Self::Previews, Self::Error>> + Send;
    /// Installs a successful snapshot before reporting elapsed time.
    fn publish_previews(
        &mut self,
        _source: &Path,
        _previews: &Self::Previews,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        std::future::ready(Ok(()))
    }

    /// Refreshes dependency metadata without analyzing or loading previews.
    fn refresh_manifest(
        &mut self,
        manifest: &Path,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send;
}

/// Retry budget for transient preview lookup failures. Analysis readiness and
/// manifest refresh are distinct stages, not subject to this retry policy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PreviewReloadPolicy {
    /// Total attempts, including the first lookup.
    pub lookup_attempts: NonZeroU8,
    /// Delay after a failed lookup, before another is attempted.
    pub retry_delay: Duration,
    /// Maximum time allowed for each individual lookup.
    pub lookup_timeout: Duration,
}

impl Default for PreviewReloadPolicy {
    fn default() -> Self {
        Self {
            lookup_attempts: NonZeroU8::new(5).expect("positive lookup budget"),
            retry_delay: Duration::from_millis(200),
            lookup_timeout: Duration::from_secs(5),
        }
    }
}

/// Outcome of one event. Only a completed source reload carries timing.
#[derive(Debug, PartialEq, Eq)]
pub enum PreviewReload<Previews> {
    /// The path cannot affect this project.
    Ignored,
    /// The coordinator stopped or its analysis context was disposed.
    Stopped,
    /// A project manifest changed and dependency metadata was refreshed.
    ManifestRefreshed(PathBuf),
    /// Source analysis and publication both completed.
    Reloaded {
        /// Source file that triggered the reload.
        source: PathBuf,
        /// The published preview snapshot.
        previews: Previews,
        /// Analysis, lookup retries, and publication; excludes queue time.
        elapsed: Duration,
    },
}

/// The stage that failed; errors do not carry a completed timing sample.
#[derive(Debug, thiserror::Error)]
pub enum PreviewReloadError<E: std::error::Error + 'static> {
    /// The selected analyzer did not finish successfully.
    #[error("preview analysis did not complete: {0}")]
    Analysis(#[source] E),
    /// The last lookup returned a backend error.
    #[error("preview lookup failed after {attempts} attempts: {source}")]
    Lookup {
        /// Number of lookup attempts made.
        attempts: u8,
        /// The last backend error.
        #[source]
        source: E,
    },
    /// The last allowed lookup timed out.
    #[error("preview lookup timed out after {attempts} attempts")]
    LookupTimeout {
        /// Number of lookup attempts made.
        attempts: u8,
    },
    /// Inspection succeeded, but the host rejected publication.
    #[error("preview snapshot could not be published: {0}")]
    Publish(#[source] E),
    /// The dependency metadata update failed.
    #[error("preview dependency metadata could not be refreshed: {0}")]
    Manifest(#[source] E),
}

/// Handles a watcher notification without starting a new watcher, analyzer,
/// or process. The host may provide an existing analysis service. Requests
/// that affect a project run one at a time; a failed request never leaks its
/// stopwatch or partial result into the next request.
pub struct PreviewReloadCoordinator<Host> {
    router: PreviewChangeRouter,
    host: Mutex<Host>,
    policy: PreviewReloadPolicy,
    stopped: watch::Sender<bool>,
}

impl<Host: PreviewReloadHost> PreviewReloadCoordinator<Host> {
    /// Creates a coordinator with the default retry policy.
    pub fn new(router: PreviewChangeRouter, host: Host) -> Self {
        Self::with_policy(router, host, PreviewReloadPolicy::default())
    }

    /// Creates a coordinator with caller-selected retry and timeout limits.
    pub fn with_policy(
        router: PreviewChangeRouter,
        host: Host,
        policy: PreviewReloadPolicy,
    ) -> Self {
        let (stopped, _) = watch::channel(false);
        Self {
            router,
            host: Mutex::new(host),
            policy,
            stopped,
        }
    }

    /// Stops active and queued reload requests, releasing their host borrows.
    /// Hosts own any external processes and their shutdown separately.
    pub fn stop(&self) {
        self.stopped.send_replace(true);
    }

    /// Routes one filesystem event and awaits its serialized outcome.
    pub async fn handle(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<PreviewReload<Host::Previews>, PreviewReloadError<Host::Error>> {
        let Some(event) = self.router.route(path) else {
            return Ok(PreviewReload::Ignored);
        };
        let mut stopped = self.stopped.subscribe();
        tokio::select! {
            biased;
            _ = stopped.wait_for(|closed| *closed) => Ok(PreviewReload::Stopped),
            result = self.process(event) => result,
        }
    }

    /// Discovers a source's previews without publishing or timing a reload.
    /// An unavailable source, stopped coordinator or disposed analysis
    /// context returns the snapshot type's empty default.
    pub async fn inspect(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<Host::Previews, PreviewReloadError<Host::Error>>
    where
        Host::Previews: Default,
    {
        let Some(PreviewChange::Source(source)) = self.router.route(path) else {
            return Ok(Host::Previews::default());
        };
        let mut stopped = self.stopped.subscribe();
        tokio::select! {
            biased;
            _ = stopped.wait_for(|closed| *closed) => Ok(Host::Previews::default()),
            result = async {
                let mut host = self.host.lock().await;
                self.lookup(&mut host, &source).await
            } => result.map(Option::unwrap_or_default),
        }
    }

    async fn process(
        &self,
        change: PreviewChange,
    ) -> Result<PreviewReload<Host::Previews>, PreviewReloadError<Host::Error>> {
        let mut host = self.host.lock().await;
        let source = match change {
            PreviewChange::Manifest(path) => {
                host.refresh_manifest(&path)
                    .await
                    .map_err(PreviewReloadError::Manifest)?;
                return Ok(PreviewReload::ManifestRefreshed(path));
            }
            PreviewChange::Source(source) => source,
        };

        let started = Instant::now();
        let Some(previews) = self.lookup(&mut host, &source).await? else {
            return Ok(PreviewReload::Stopped);
        };
        host.publish_previews(&source, &previews)
            .await
            .map_err(PreviewReloadError::Publish)?;
        Ok(PreviewReload::Reloaded {
            source,
            previews,
            elapsed: started.elapsed(),
        })
    }

    async fn lookup(
        &self,
        host: &mut Host,
        source: &Path,
    ) -> Result<Option<Host::Previews>, PreviewReloadError<Host::Error>> {
        if host.analysis_disposed() {
            return Ok(None);
        }
        let ready = host.wait_for_analysis().await;
        if host.analysis_disposed() {
            return Ok(None);
        }
        ready.map_err(PreviewReloadError::Analysis)?;
        let attempts = self.policy.lookup_attempts.get();
        for attempt in 1..=attempts {
            if host.analysis_disposed() {
                return Ok(None);
            }
            let found = timeout(self.policy.lookup_timeout, host.load_previews(source)).await;
            if host.analysis_disposed() {
                return Ok(None);
            }
            match found {
                Ok(Ok(previews)) => return Ok(Some(previews)),
                Ok(Err(source)) if attempt == attempts => {
                    return Err(PreviewReloadError::Lookup { attempts, source });
                }
                Err(_) if attempt == attempts => {
                    return Err(PreviewReloadError::LookupTimeout { attempts });
                }
                _ => sleep(self.policy.retry_delay).await,
            }
        }
        unreachable!("the nonzero attempt budget always produces an outcome")
    }
}
