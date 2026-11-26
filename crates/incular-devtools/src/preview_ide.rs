//! Readiness and request ordering for an already-connected preview IDE service.

use std::{
    future::Future,
    path::Path,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use futures_util::{
    FutureExt, StreamExt,
    future::{BoxFuture, WeakShared},
    stream::BoxStream,
};

/// Capabilities registered together by a preview analysis service.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PreviewIdeCapability {
    /// Workspace-wide preview lookup.
    WorkspacePreviews,
    /// Preview lookup for a single document.
    DocumentPreviews,
    /// Waiting for the current workspace analysis to complete.
    AnalysisComplete,
}

impl PreviewIdeCapability {
    const REQUIRED: [Self; 3] = [
        Self::WorkspacePreviews,
        Self::DocumentPreviews,
        Self::AnalysisComplete,
    ];
}

/// Result of requesting preview analysis events on the remote connection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PreviewStreamStatus {
    /// The connection requested the stream for the first time.
    Subscribed,
    /// The connection already requested the stream.
    AlreadySubscribed,
}

/// Relevant events emitted by the connected preview analysis service.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PreviewIdeEvent {
    /// The service has finished registering all preview capabilities.
    Initialized,
    /// Another analysis event, not a signal that discovery has finished.
    Other,
}

/// A connected IDE transport. Implementations use the same connection for
/// capability discovery, events, and requests.
pub trait PreviewIdeTransport: Send + Sync + 'static {
    /// The preview list returned by the analyzer.
    type Previews;
    /// Transport or analyzer request failure.
    type Error: std::error::Error + Send + Sync + 'static;

    /// Attaches a local listener before asking the remote service to stream
    /// events. Dropping the stream must detach this local listener.
    fn events(&self) -> BoxStream<'static, Result<PreviewIdeEvent, Self::Error>>;
    /// Requests remote event delivery. AlreadySubscribed is not an error.
    fn subscribe(&self) -> impl Future<Output = Result<PreviewStreamStatus, Self::Error>> + Send;
    /// Queries registered capabilities for the preview analysis service.
    fn capabilities(
        &self,
    ) -> impl Future<Output = Result<Vec<PreviewIdeCapability>, Self::Error>> + Send;
    /// Gets previews for the workspace, or for just the requested document.
    fn previews(
        &self,
        document: Option<&Path>,
    ) -> impl Future<Output = Result<Self::Previews, Self::Error>> + Send;
    /// Requests the analyzer's analysis completion barrier.
    fn analysis_complete(&self) -> impl Future<Output = Result<(), Self::Error>> + Send;
}

/// Failed readiness or a failed request on the preview IDE connection.
#[derive(Debug, thiserror::Error)]
pub enum PreviewIdeError<E: std::error::Error + 'static> {
    /// The IDE transport returned the underlying error.
    #[error("preview IDE transport failed: {0}")]
    Transport(#[source] Arc<E>),
    /// The event stream closed before the analysis service became ready.
    #[error("preview IDE event stream closed before initialization")]
    Closed,
    /// The required preview methods did not become ready in time.
    #[error("preview IDE initialization timed out")]
    InitializationTimeout,
}

impl<E: std::error::Error + 'static> Clone for PreviewIdeError<E> {
    fn clone(&self) -> Self {
        match self {
            Self::Transport(error) => Self::Transport(error.clone()),
            Self::Closed => Self::Closed,
            Self::InitializationTimeout => Self::InitializationTimeout,
        }
    }
}

impl<E: std::error::Error + 'static> From<E> for PreviewIdeError<E> {
    fn from(error: E) -> Self {
        Self::Transport(Arc::new(error))
    }
}

type Attempt<E> = BoxFuture<'static, Result<(), PreviewIdeError<E>>>;

/// Client for one existing preview analysis connection. Initialization is
/// shared by concurrent callers; this type never starts another analyzer.
pub struct PreviewIde<T: PreviewIdeTransport> {
    transport: Arc<T>,
    ready: Arc<AtomicBool>,
    in_flight: Mutex<Option<WeakShared<Attempt<T::Error>>>>,
    initialization_timeout: Duration,
}

impl<T: PreviewIdeTransport> PreviewIde<T> {
    /// Uses the pinned reference's 30-second initialization-event timeout.
    pub fn new(transport: T) -> Self {
        Self::with_initialization_timeout(transport, Duration::from_secs(30))
    }

    /// Sets the maximum wait for an initialization event after registration
    /// has been queried. Individual RPC deadlines belong to the transport.
    pub fn with_initialization_timeout(transport: T, timeout: Duration) -> Self {
        Self {
            transport: Arc::new(transport),
            ready: Arc::new(AtomicBool::new(false)),
            in_flight: Mutex::new(None),
            initialization_timeout: timeout,
        }
    }

    /// Queries workspace previews, without starting a new analysis process.
    pub async fn workspace_previews(&self) -> Result<T::Previews, PreviewIdeError<T::Error>> {
        self.ensure_ready().await?;
        Ok(self.transport.previews(None).await?)
    }

    /// Queries previews for one document through the same ready connection.
    pub async fn document_previews(
        &self,
        document: &Path,
    ) -> Result<T::Previews, PreviewIdeError<T::Error>> {
        self.ensure_ready().await?;
        Ok(self.transport.previews(Some(document)).await?)
    }

    /// Waits for current analysis, using the reference's 100 ms settle delay.
    pub async fn wait_for_analysis(&self) -> Result<(), PreviewIdeError<T::Error>> {
        self.wait_for_analysis_after(Duration::from_millis(100))
            .await
    }

    /// Waits for current analysis after a caller-selected settle delay.
    pub async fn wait_for_analysis_after(
        &self,
        delay: Duration,
    ) -> Result<(), PreviewIdeError<T::Error>> {
        self.ensure_ready().await?;
        if !delay.is_zero() {
            tokio::time::sleep(delay).await;
        }
        Ok(self.transport.analysis_complete().await?)
    }

    async fn ensure_ready(&self) -> Result<(), PreviewIdeError<T::Error>> {
        let attempt = {
            let mut slot = self.in_flight.lock().unwrap();
            if self.ready.load(Ordering::Acquire) {
                return Ok(());
            }
            match slot.as_ref().and_then(WeakShared::upgrade) {
                Some(attempt) => attempt,
                None => {
                    let attempt = Self::initialize(
                        self.transport.clone(),
                        self.ready.clone(),
                        self.initialization_timeout,
                    )
                    .boxed()
                    .shared();
                    // The client does not hold the task alive. Canceling the
                    // last waiter promptly releases the event subscription.
                    *slot = attempt.downgrade();
                    attempt
                }
            }
        };
        attempt.await
    }

    async fn initialize(
        transport: Arc<T>,
        ready: Arc<AtomicBool>,
        timeout: Duration,
    ) -> Result<(), PreviewIdeError<T::Error>> {
        let mut events = transport.events();
        let _ = transport.subscribe().await?;
        let registered = transport.capabilities().await?;
        if !PreviewIdeCapability::REQUIRED
            .iter()
            .all(|capability| registered.contains(capability))
        {
            tokio::time::timeout(timeout, async {
                while let Some(event) = events.next().await {
                    if event? == PreviewIdeEvent::Initialized {
                        return Ok(());
                    }
                }
                Err(PreviewIdeError::Closed)
            })
            .await
            .map_err(|_| PreviewIdeError::InitializationTimeout)??;
        }
        drop(events);
        ready.store(true, Ordering::Release);
        Ok(())
    }
}
