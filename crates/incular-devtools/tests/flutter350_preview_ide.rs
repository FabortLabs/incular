//! Preview IDE readiness behavior with a controlled transport.
use std::{
    io,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    task::Poll,
    time::Duration,
};

use futures_util::{StreamExt, poll, stream};
use incular_devtools::preview_ide::{
    PreviewIde, PreviewIdeCapability as Cap, PreviewIdeError, PreviewIdeEvent, PreviewIdeTransport,
    PreviewStreamStatus,
};
use tokio::sync::mpsc;

type Event = Result<PreviewIdeEvent, io::Error>;

#[derive(Default)]
struct Trace {
    event_tx: Option<mpsc::UnboundedSender<Event>>,
    subscriptions: usize,
    registry_queries: usize,
    registry: Vec<Cap>,
    requested_documents: Vec<Option<PathBuf>>,
    preview_result: Vec<PathBuf>,
    barriers: usize,
    already_subscribed: bool,
    refuse_subscribe: bool,
}

#[derive(Clone, Default)]
struct Connection(Arc<Mutex<Trace>>);

impl Connection {
    fn emit(&self, event: PreviewIdeEvent) {
        self.0
            .lock()
            .unwrap()
            .event_tx
            .as_ref()
            .unwrap()
            .send(Ok(event))
            .unwrap();
    }

    fn listener_is_attached(&self) -> bool {
        self.0
            .lock()
            .unwrap()
            .event_tx
            .as_ref()
            .is_some_and(|tx| !tx.is_closed())
    }

    fn request_counts(&self) -> (usize, usize) {
        let trace = self.0.lock().unwrap();
        (trace.subscriptions, trace.registry_queries)
    }

    fn ready() -> Self {
        let connection = Self::default();
        connection.0.lock().unwrap().registry = vec![
            Cap::WorkspacePreviews,
            Cap::DocumentPreviews,
            Cap::AnalysisComplete,
        ];
        connection
    }
}

impl PreviewIdeTransport for Connection {
    type Previews = Vec<PathBuf>;
    type Error = io::Error;

    fn events(&self) -> stream::BoxStream<'static, Event> {
        let (tx, rx) = mpsc::unbounded_channel();
        let mut trace = self.0.lock().unwrap();
        assert!(
            trace.event_tx.as_ref().is_none_or(|old| old.is_closed()),
            "duplicate listener"
        );
        trace.event_tx = Some(tx);
        stream::unfold(rx, |mut rx| async move {
            rx.recv().await.map(|event| (event, rx))
        })
        .boxed()
    }

    async fn subscribe(&self) -> io::Result<PreviewStreamStatus> {
        let mut trace = self.0.lock().unwrap();
        assert!(
            !trace.event_tx.as_ref().unwrap().is_closed(),
            "listen locally before subscribing"
        );
        trace.subscriptions += 1;
        if trace.refuse_subscribe {
            return Err(io::ErrorKind::ConnectionRefused.into());
        }
        Ok(if trace.already_subscribed {
            PreviewStreamStatus::AlreadySubscribed
        } else {
            trace.already_subscribed = true;
            PreviewStreamStatus::Subscribed
        })
    }

    async fn capabilities(&self) -> io::Result<Vec<Cap>> {
        let mut trace = self.0.lock().unwrap();
        trace.registry_queries += 1;
        Ok(trace.registry.clone())
    }

    async fn previews(&self, document: Option<&Path>) -> io::Result<Self::Previews> {
        let mut trace = self.0.lock().unwrap();
        trace.requested_documents.push(document.map(Path::to_owned));
        Ok(trace.preview_result.clone())
    }

    async fn analysis_complete(&self) -> io::Result<()> {
        self.0.lock().unwrap().barriers += 1;
        Ok(())
    }
}

#[tokio::test]
async fn concurrent_workspace_requests_wait_on_one_initialization() {
    let connection = Connection::default();
    let ide = PreviewIde::new(connection.clone());
    let mut first = Box::pin(ide.workspace_previews());
    let mut second = Box::pin(ide.workspace_previews());
    assert!(poll!(&mut first).is_pending());
    assert!(poll!(&mut second).is_pending());
    assert_eq!(connection.request_counts(), (1, 1));
    connection.emit(PreviewIdeEvent::Initialized);
    let (first, second) = tokio::join!(first, second);
    assert!(first.unwrap().is_empty());
    assert!(second.unwrap().is_empty());
    assert_eq!(connection.request_counts(), (1, 1));
    assert!(!connection.listener_is_attached());
}

#[tokio::test]
async fn workspace_and_document_requests_share_the_listener_and_preserve_document() {
    let connection = Connection::default();
    let ide = PreviewIde::new(connection.clone());
    let document = Path::new("src/main.rs");
    let mut workspace = Box::pin(ide.workspace_previews());
    let mut single = Box::pin(ide.document_previews(document));
    assert!(poll!(&mut workspace).is_pending());
    assert!(poll!(&mut single).is_pending());
    connection.emit(PreviewIdeEvent::Initialized);
    assert!(workspace.await.unwrap().is_empty());
    assert!(single.await.unwrap().is_empty());
    assert_eq!(connection.request_counts(), (1, 1));
    assert_eq!(
        connection.0.lock().unwrap().requested_documents,
        [None, Some(document.to_owned())]
    );
}

#[tokio::test]
async fn tolerates_a_stream_already_registered_by_the_connected_ide() {
    let connection = Connection::default();
    connection.0.lock().unwrap().already_subscribed = true;
    let ide = PreviewIde::new(connection.clone());
    let mut request = Box::pin(ide.workspace_previews());
    assert!(poll!(&mut request).is_pending());
    connection.emit(PreviewIdeEvent::Initialized);
    assert!(request.await.unwrap().is_empty());
    assert_eq!(connection.request_counts(), (1, 1));
}

#[tokio::test]
async fn analysis_uses_the_connected_transport_barrier_only_after_readiness() {
    let connection = Connection::default();
    let ide = PreviewIde::new(connection.clone());
    let mut wait = Box::pin(ide.wait_for_analysis_after(Duration::ZERO));
    assert!(poll!(&mut wait).is_pending());
    assert_eq!(connection.0.lock().unwrap().barriers, 0);
    connection.emit(PreviewIdeEvent::Initialized);
    wait.await.unwrap();
    let trace = connection.0.lock().unwrap();
    assert_eq!(trace.barriers, 1);
    assert!(trace.requested_documents.is_empty());
}

#[tokio::test]
async fn incomplete_registry_and_unrelated_events_do_not_unblock_lookup() {
    let connection = Connection::default();
    connection.0.lock().unwrap().registry = vec![Cap::WorkspacePreviews];
    let ide = PreviewIde::new(connection.clone());
    let mut request = Box::pin(ide.workspace_previews());
    assert!(poll!(&mut request).is_pending());
    connection.emit(PreviewIdeEvent::Other);
    assert!(poll!(&mut request).is_pending());
    assert!(connection.0.lock().unwrap().requested_documents.is_empty());
    connection.emit(PreviewIdeEvent::Initialized);
    assert!(request.await.unwrap().is_empty());
}

#[tokio::test]
async fn complete_registry_proceeds_without_an_event_and_is_cached() {
    let connection = Connection::ready();
    let ide = PreviewIde::new(connection.clone());
    let mut request = Box::pin(ide.workspace_previews());
    assert!(matches!(poll!(&mut request), Poll::Ready(Ok(result)) if result.is_empty()));
    assert!(ide.workspace_previews().await.unwrap().is_empty());
    assert_eq!(connection.request_counts(), (1, 1));
    assert!(!connection.listener_is_attached());
}

#[tokio::test]
async fn transport_subscription_error_detaches_the_listener_and_allows_retry() {
    let connection = Connection::ready();
    connection.0.lock().unwrap().refuse_subscribe = true;
    let ide = PreviewIde::new(connection.clone());
    assert!(matches!(
        ide.workspace_previews().await,
        Err(PreviewIdeError::Transport(error)) if error.kind() == io::ErrorKind::ConnectionRefused
    ));
    assert!(!connection.listener_is_attached());
    assert_eq!(connection.request_counts(), (1, 0));
    connection.0.lock().unwrap().refuse_subscribe = false;
    assert!(ide.workspace_previews().await.unwrap().is_empty());
    assert_eq!(connection.request_counts(), (2, 1));
}

#[tokio::test]
async fn canceling_the_last_waiter_releases_the_listener_without_tearing_down_the_transport() {
    let connection = Connection::default();
    let ide = PreviewIde::new(connection.clone());
    let mut first = Box::pin(ide.workspace_previews());
    let mut second = Box::pin(ide.workspace_previews());
    assert!(poll!(&mut first).is_pending());
    assert!(poll!(&mut second).is_pending());
    drop(first);
    assert!(connection.listener_is_attached());
    drop(second);
    assert!(!connection.listener_is_attached());

    let mut retry = Box::pin(ide.workspace_previews());
    assert!(poll!(&mut retry).is_pending());
    connection.emit(PreviewIdeEvent::Initialized);
    assert!(retry.await.unwrap().is_empty());
    assert_eq!(connection.request_counts(), (2, 2));
}

#[tokio::test]
async fn initialization_deadline_releases_the_listener_and_does_not_issue_preview_rpc() {
    let connection = Connection::default();
    let ide = PreviewIde::with_initialization_timeout(connection.clone(), Duration::ZERO);
    assert!(matches!(
        ide.workspace_previews().await,
        Err(PreviewIdeError::InitializationTimeout)
    ));
    assert!(!connection.listener_is_attached());
    assert!(connection.0.lock().unwrap().requested_documents.is_empty());
}

#[tokio::test]
async fn connected_ide_supplies_both_an_explicit_barrier_and_a_source_reload() {
    use incular_devtools::{
        preview_ide::PreviewIdeReloadHost,
        preview_manifest::{PreviewManifest, PreviewRegeneration},
        preview_reload::{PreviewReload, PreviewReloadCoordinator},
    };

    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let cargo = root.join("Cargo.toml");
    let source = root.join("src/lib.rs");
    std::fs::create_dir(root.join("src")).unwrap();
    std::fs::write(&cargo, "[package]\nname='preview'\nversion='0.1.0'").unwrap();
    let manifest = PreviewManifest::new(root.join("preview-out"), "pinned", &cargo);
    manifest.generate().unwrap();

    let connection = Connection::ready();
    connection.0.lock().unwrap().preview_result = vec![root.join("preview-out/sample.rs")];
    let results = Arc::new(Mutex::new(Vec::new()));
    let snapshot = results.clone();
    let host = PreviewIdeReloadHost::new(
        PreviewIde::new(connection.clone()),
        manifest.clone(),
        move |source: &Path, previews: &Vec<PathBuf>| {
            snapshot
                .lock()
                .unwrap()
                .push((source.to_owned(), previews.clone()));
            Ok::<(), io::Error>(())
        },
    );

    assert_eq!(connection.request_counts(), (0, 0));
    assert_eq!(connection.0.lock().unwrap().barriers, 0);
    host.ide()
        .wait_for_analysis_after(Duration::ZERO)
        .await
        .unwrap();
    assert_eq!(connection.0.lock().unwrap().barriers, 1);

    let coordinator = PreviewReloadCoordinator::new(manifest.change_router().unwrap(), host);
    std::fs::write(&source, "fn preview() {}").unwrap();
    let result = coordinator.handle(&source).await.unwrap();
    let expected = vec![root.join("preview-out/sample.rs")];
    assert!(
        matches!(result, PreviewReload::Reloaded { source: seen, previews, .. } if seen == source && previews == expected)
    );
    assert_eq!(*results.lock().unwrap(), [(source.clone(), expected)]);
    assert_eq!(connection.0.lock().unwrap().barriers, 2);

    std::fs::write(&cargo, "[package]\nname='preview'\nversion='0.2.0'").unwrap();
    assert_eq!(
        coordinator.handle(&cargo).await.unwrap(),
        PreviewReload::ManifestRefreshed(cargo)
    );
    assert_eq!(manifest.regeneration().unwrap(), PreviewRegeneration::None);
    let trace = connection.0.lock().unwrap();
    assert_eq!(trace.barriers, 2);
    assert_eq!(trace.requested_documents, [Some(source)]);
    assert_eq!((trace.subscriptions, trace.registry_queries), (1, 1));
}
