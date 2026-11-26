use std::{
    convert::Infallible,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
        mpsc,
    },
    thread,
    time::Duration,
};

use incular_assets::{AssetBundle, AssetError, CachingAssetBundle};

struct DeferredBundle {
    reads: Arc<AtomicUsize>,
    first_started: mpsc::SyncSender<()>,
    finish_first: Mutex<mpsc::Receiver<()>>,
}

impl AssetBundle for DeferredBundle {
    fn load(&self, key: &str) -> Result<Arc<[u8]>, AssetError> {
        let read = self.reads.fetch_add(1, Ordering::Relaxed);
        if read == 0 {
            self.first_started.send(()).unwrap();
            self.finish_first
                .lock()
                .unwrap()
                .recv_timeout(Duration::from_secs(3))
                .expect("test never resumed the old asset read");
        }
        Ok(Arc::from(format!("{key}:{read}").into_bytes()))
    }
}

fn exercise(structured: bool, all: bool) {
    let reads = Arc::new(AtomicUsize::new(0));
    let (first_started, wait_for_read) = mpsc::sync_channel(0);
    let (finish_first, worker_gate) = mpsc::sync_channel(0);
    let root = Arc::new(CachingAssetBundle::new(DeferredBundle {
        reads: reads.clone(),
        first_started,
        finish_first: Mutex::new(worker_gate),
    }));
    let get_asset = move |bundle: &CachingAssetBundle<DeferredBundle>| {
        if structured {
            bundle
                .load_structured_data("fixture", |text| Ok::<_, Infallible>(text.to_owned()))
                .unwrap()
        } else {
            bundle.load_string("fixture", true).unwrap().to_string()
        }
    };
    let worker = thread::spawn({
        let root = root.clone();
        move || get_asset(&root)
    });
    wait_for_read.recv_timeout(Duration::from_secs(3)).unwrap();
    if all {
        root.clear();
    } else {
        root.evict("fixture");
    }
    finish_first.send(()).unwrap();
    assert_eq!(
        worker.join().unwrap(),
        "fixture:0",
        "callers still get their original result"
    );
    assert_eq!(
        get_asset(&root),
        "fixture:1",
        "an older request must not refill the cache"
    );
    assert_eq!(get_asset(&root), "fixture:1");
    assert_eq!(reads.load(Ordering::Relaxed), 2);
}

#[test]
fn clearing_cannot_be_undone_by_in_flight_string_or_structured_reads() {
    exercise(false, true);
    exercise(true, true);
}

#[test]
fn evicting_cannot_be_undone_by_in_flight_string_or_structured_reads() {
    exercise(false, false);
    exercise(true, false);
}
