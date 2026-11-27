use std::{
    fs,
    future::pending,
    io,
    num::NonZeroU8,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::Duration,
};

use incular_devtools::{
    preview_manifest::{PreviewManifest, PreviewRegeneration},
    preview_reload::{
        PreviewReload, PreviewReloadCoordinator, PreviewReloadError, PreviewReloadHost,
        PreviewReloadPolicy,
    },
};
use tokio::sync::Notify;

#[derive(Default)]
struct Trace {
    analysis: usize,
    lookup: Vec<PathBuf>,
    published: Vec<(PathBuf, String)>,
    publish_error: bool,
    manifests: Vec<PathBuf>,
    analysis_disposed: bool,
    dispose_after_lookup: bool,
    analysis_error: bool,
    lookup_failures: usize,
    hang_lookup: bool,
    lookup_entered: Option<Arc<Notify>>,
}

struct Host {
    trace: Arc<Mutex<Trace>>,
    manifest: PreviewManifest,
}

impl PreviewReloadHost for Host {
    type Previews = String;
    type Error = io::Error;

    fn analysis_disposed(&self) -> bool {
        self.trace.lock().unwrap().analysis_disposed
    }

    async fn wait_for_analysis(&mut self) -> io::Result<()> {
        let mut trace = self.trace.lock().unwrap();
        trace.analysis += 1;
        if trace.analysis_error {
            Err(io::Error::other("analysis not ready"))
        } else {
            Ok(())
        }
    }

    async fn load_previews(&mut self, source: &Path) -> io::Result<Self::Previews> {
        let hang = {
            let mut trace = self.trace.lock().unwrap();
            assert!(
                !trace.analysis_disposed,
                "disposed analysis must not be entered"
            );
            trace.lookup.push(source.to_owned());
            if trace.dispose_after_lookup {
                trace.analysis_disposed = true;
            }
            if trace.lookup_failures > 0 {
                trace.lookup_failures -= 1;
                return Err(io::Error::other("preview service unavailable"));
            }
            if let Some(notify) = &trace.lookup_entered {
                notify.notify_one();
            }
            trace.hang_lookup
        };
        if hang {
            return pending().await;
        }
        fs::read_to_string(source)
    }

    async fn publish_previews(
        &mut self,
        source: &Path,
        previews: &Self::Previews,
    ) -> io::Result<()> {
        let mut trace = self.trace.lock().unwrap();
        if trace.publish_error {
            return Err(io::Error::other("preview snapshot not accepted"));
        }
        trace.published.push((source.to_owned(), previews.clone()));
        Ok(())
    }

    async fn refresh_manifest(&mut self, manifest: &Path) -> io::Result<()> {
        self.trace
            .lock()
            .unwrap()
            .manifests
            .push(manifest.to_owned());
        self.manifest
            .update_project_hashes()
            .map_err(io::Error::other)?;
        Ok(())
    }
}

#[tokio::test]
async fn a_late_watcher_event_does_not_reenter_disposed_analysis() {
    let f = Fixture::new();
    let path = f.change("src/foo.rs", "preview factory");
    f.trace.lock().unwrap().analysis_disposed = true;
    assert_eq!(
        f.coordinator.handle(&path).await.unwrap(),
        PreviewReload::Stopped
    );
    let trace = f.trace.lock().unwrap();
    assert_eq!(trace.analysis, 0);
    assert!(trace.lookup.is_empty());
    assert!(trace.published.is_empty());
}

#[tokio::test]
async fn direct_inspection_after_disposal_returns_an_empty_snapshot_without_publication() {
    let f = Fixture::new();
    let path = f.change("src/foo.rs", "preview factory");
    assert_eq!(
        f.coordinator.inspect(&path).await.unwrap(),
        "preview factory"
    );
    {
        let mut trace = f.trace.lock().unwrap();
        assert_eq!(trace.analysis, 1);
        assert_eq!(trace.lookup, [path.clone()]);
        assert!(trace.published.is_empty());
        trace.analysis_disposed = true;
    }
    assert_eq!(f.coordinator.inspect(&path).await.unwrap(), "");
    let trace = f.trace.lock().unwrap();
    assert_eq!(trace.analysis, 1);
    assert_eq!(trace.lookup, [path]);
    assert!(trace.published.is_empty());
}

#[tokio::test]
async fn disposal_during_lookup_discards_the_snapshot_before_publication() {
    let f = Fixture::new();
    let path = f.change("src/foo.rs", "not safe to publish");
    f.trace.lock().unwrap().dispose_after_lookup = true;
    assert_eq!(
        f.coordinator.handle(&path).await.unwrap(),
        PreviewReload::Stopped
    );
    let trace = f.trace.lock().unwrap();
    assert_eq!(trace.lookup, [path]);
    assert!(trace.published.is_empty());
}

struct Fixture {
    dir: tempfile::TempDir,
    trace: Arc<Mutex<Trace>>,
    coordinator: PreviewReloadCoordinator<Host>,
}

impl Fixture {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let manifest_path = dir.path().join("Cargo.toml");
        fs::write(
            &manifest_path,
            "[package]\nname = 'preview-test'\nversion = '0.1.0'",
        )
        .unwrap();
        fs::create_dir(dir.path().join("src")).unwrap();
        let manifest = PreviewManifest::new(
            dir.path().join("src/generated-preview"),
            "pinned",
            manifest_path,
        );
        manifest.generate().unwrap();
        let router = manifest.change_router().unwrap();
        let trace = Arc::new(Mutex::new(Trace::default()));
        let coordinator = PreviewReloadCoordinator::with_policy(
            router,
            Host {
                trace: trace.clone(),
                manifest,
            },
            PreviewReloadPolicy {
                retry_delay: Duration::ZERO,
                lookup_timeout: Duration::from_millis(50),
                ..PreviewReloadPolicy::default()
            },
        );
        Self {
            dir,
            trace,
            coordinator,
        }
    }

    fn change(&self, path: &str, contents: &str) -> PathBuf {
        let path = self.dir.path().join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, contents).unwrap();
        path
    }
}

#[test]
fn retry_policy_matches_the_reference_budget() {
    let policy = PreviewReloadPolicy::default();
    assert_eq!(policy.lookup_attempts, NonZeroU8::new(5).unwrap());
    assert_eq!(policy.retry_delay, Duration::from_millis(200));
    assert_eq!(policy.lookup_timeout, Duration::from_secs(5));
}

#[tokio::test]
async fn two_source_writes_each_return_fresh_previews_and_reload_timing() {
    let f = Fixture::new();
    for text in ["fn setup() {}", "fn setup_updated() {}"] {
        let source = f.change("src/lib.rs", text);
        let PreviewReload::Reloaded {
            source: seen,
            previews,
            ..
        } = f.coordinator.handle(&source).await.unwrap()
        else {
            panic!("expected a successful, timed reload")
        };
        assert_eq!(seen, source);
        assert_eq!(previews, text);
    }
    let trace = f.trace.lock().unwrap();
    assert_eq!(trace.analysis, 2);
    assert_eq!(trace.lookup.len(), 2);
    assert_eq!(
        trace
            .published
            .iter()
            .map(|(_, preview)| preview.as_str())
            .collect::<Vec<_>>(),
        ["fn setup() {}", "fn setup_updated() {}"]
    );
    assert!(trace.manifests.is_empty());
}

#[tokio::test]
async fn cargo_manifest_refresh_does_not_run_preview_analysis_or_emit_timing() {
    let f = Fixture::new();
    let path = f.change(
        "Cargo.toml",
        "[package]\nname = 'preview-test'\nversion = '0.2.0'",
    );
    assert_eq!(
        f.coordinator.handle(&path).await.unwrap(),
        PreviewReload::ManifestRefreshed(path.clone())
    );
    let trace = f.trace.lock().unwrap();
    assert_eq!(trace.analysis, 0);
    assert!(trace.lookup.is_empty());
    assert_eq!(trace.manifests, [path.clone()]);
    let regenerated =
        PreviewManifest::new(f.dir.path().join("src/generated-preview"), "pinned", path);
    assert_eq!(
        regenerated.regeneration().unwrap(),
        PreviewRegeneration::None
    );
}

#[tokio::test]
async fn non_sources_and_ignored_directories_do_not_touch_the_host() {
    let f = Fixture::new();
    for path in [
        "README.md",
        "src/description.txt",
        "target/generated.rs",
        "src/generated-preview/src/main.rs",
        "src/generated-preview/Cargo.toml",
    ] {
        let entry = f.change(path, "not a preview source");
        assert_eq!(
            f.coordinator.handle(&entry).await.unwrap(),
            PreviewReload::Ignored,
            "{path}"
        );
    }
    let trace = f.trace.lock().unwrap();
    assert_eq!(trace.analysis, 0);
    assert!(trace.lookup.is_empty() && trace.manifests.is_empty());
}

#[tokio::test]
async fn a_lookup_failure_spends_five_attempts_and_the_next_write_is_fresh() {
    let f = Fixture::new();
    f.trace.lock().unwrap().lookup_failures = 5;
    let source = f.change("src/lib.rs", "stale");
    assert!(matches!(
        f.coordinator.handle(&source).await,
        Err(PreviewReloadError::Lookup { attempts: 5, .. })
    ));
    {
        let trace = f.trace.lock().unwrap();
        assert_eq!(trace.analysis, 1);
        assert_eq!(trace.lookup, vec![source.clone(); 5]);
    }
    f.change("src/lib.rs", "recovered");
    assert!(matches!(
        f.coordinator.handle(&source).await.unwrap(),
        PreviewReload::Reloaded { previews, .. } if previews == "recovered"
    ));
    let trace = f.trace.lock().unwrap();
    assert_eq!(trace.analysis, 2);
    assert_eq!(trace.lookup.len(), 6);
}

#[tokio::test]
async fn analysis_failure_does_not_start_lookup_and_the_next_event_can_succeed() {
    let f = Fixture::new();
    f.trace.lock().unwrap().analysis_error = true;
    let source = f.change("src/lib.rs", "unready");
    assert!(matches!(
        f.coordinator.handle(&source).await,
        Err(PreviewReloadError::Analysis(_))
    ));
    {
        let mut trace = f.trace.lock().unwrap();
        assert_eq!(trace.analysis, 1);
        assert!(trace.lookup.is_empty());
        trace.analysis_error = false;
    }
    f.change("src/lib.rs", "ready");
    assert!(matches!(
        f.coordinator.handle(&source).await.unwrap(),
        PreviewReload::Reloaded { previews, .. } if previews == "ready"
    ));
}

#[tokio::test]
async fn shutting_down_drops_inflight_and_queued_host_work_without_timing() {
    let f = Fixture::new();
    let entered = Arc::new(Notify::new());
    {
        let mut trace = f.trace.lock().unwrap();
        trace.hang_lookup = true;
        trace.lookup_entered = Some(entered.clone());
    }
    let source = f.change("src/lib.rs", "first");
    let other = f.change("src/mod.rs", "second");
    let (active, queued, ()) = tokio::join!(
        biased;
        f.coordinator.handle(&source),
        f.coordinator.handle(&other),
        async {
            entered.notified().await;
            f.coordinator.stop();
        },
    );
    assert_eq!(active.unwrap(), PreviewReload::Stopped);
    assert_eq!(queued.unwrap(), PreviewReload::Stopped);
    let trace = f.trace.lock().unwrap();
    assert_eq!(trace.analysis, 1);
    assert_eq!(trace.lookup, [source]);
}

#[tokio::test]
async fn an_unresponsive_preview_backend_has_a_bounded_retry_budget() {
    let f = Fixture::new();
    f.trace.lock().unwrap().hang_lookup = true;
    let source = f.change("src/lib.rs", "unresponsive");
    let result = tokio::time::timeout(Duration::from_secs(2), f.coordinator.handle(&source))
        .await
        .expect("the coordinator must bound all lookup attempts");
    assert!(matches!(
        result,
        Err(PreviewReloadError::LookupTimeout { attempts: 5 })
    ));
    let mut trace = f.trace.lock().unwrap();
    assert_eq!(trace.analysis, 1);
    assert_eq!(trace.lookup, vec![source; 5]);
    trace.hang_lookup = false;
}

#[tokio::test]
async fn publishing_a_preview_is_part_of_a_successful_reload() {
    let f = Fixture::new();
    f.trace.lock().unwrap().publish_error = true;
    let source = f.change("src/lib.rs", "unpublished");
    assert!(matches!(
        f.coordinator.handle(&source).await,
        Err(PreviewReloadError::Publish(_)),
    ));
    {
        let mut trace = f.trace.lock().unwrap();
        assert_eq!(trace.lookup, [source.clone()]);
        assert!(trace.published.is_empty());
        trace.publish_error = false;
    }
    f.change("src/lib.rs", "freshly published");
    assert!(matches!(
        f.coordinator.handle(&source).await.unwrap(),
        PreviewReload::Reloaded { previews, .. } if previews == "freshly published",
    ));
    assert_eq!(
        f.trace.lock().unwrap().published,
        [(source, "freshly published".to_owned())],
    );
}

#[cfg(feature = "preview-watch")]
#[tokio::test]
async fn native_change_reloads_and_publishes_the_file_contents() {
    use incular_devtools::{
        preview_changes::PreviewChange,
        preview_watcher::{PreviewWatchUpdate, PreviewWatcher},
    };

    let f = Fixture::new();
    let manifest = PreviewManifest::new(
        f.dir.path().join("src/generated-preview"),
        "pinned",
        f.dir.path().join("Cargo.toml"),
    );
    let mut watcher = PreviewWatcher::start(manifest.change_router().unwrap()).unwrap();
    f.change("src/generated-preview/src/lib.rs", "not a preview");
    let content = "fn real_preview() {}";
    let updated = f.change("src/lib.rs", content);
    let notification = tokio::time::timeout(Duration::from_secs(5), watcher.next_update())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    let PreviewWatchUpdate::Change(PreviewChange::Source(path)) = notification else {
        panic!("expected a source invalidation, got {notification:?}");
    };
    assert_eq!(path, updated);
    let result = f.coordinator.handle(&path).await.unwrap();
    assert!(matches!(result, PreviewReload::Reloaded { ref previews, .. } if previews == content));
    assert_eq!(
        f.trace.lock().unwrap().published,
        [(updated, content.to_owned())]
    );
}
