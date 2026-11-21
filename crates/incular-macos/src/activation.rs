//! Open-file, open-URL and reopen events, added to the event loop's AppKit
//! application delegate.

use incular_platform::{ApplicationActivation, DocumentActivation, UrlActivation};
use objc2::{
    runtime::{AnyClass, AnyObject, Bool, ClassBuilder, Sel},
    sel,
};
use objc2_app_kit::NSApplication;
use objc2_foundation::{MainThreadMarker, NSArray, NSString, NSURL};
use std::{
    cell::{Cell, RefCell},
    path::PathBuf,
    rc::Rc,
    sync::Arc,
};
use url::Url;

type Deliver = Arc<dyn Fn(ApplicationActivation) + Send + Sync>;

thread_local! {
    /// Receiver for activations; AppKit calls the delegate on the main thread.
    static DELIVER: RefCell<Option<Deliver>> = const { RefCell::new(None) };
}

fn deliver(activation: ApplicationActivation) {
    // Clone out first so a receiver that restarts the watch can't re-borrow.
    if let Some(deliver) = DELIVER.with_borrow(Clone::clone) {
        deliver(activation);
    }
}

extern "C" fn application_open_urls(
    _this: &AnyObject,
    _cmd: Sel,
    _application: &NSApplication,
    urls: &NSArray<NSURL>,
) {
    for activation in normalize_urls(urls) {
        deliver(activation);
    }
}

extern "C" fn application_open_files(
    _this: &AnyObject,
    _cmd: Sel,
    _application: &NSApplication,
    filenames: &NSArray<NSString>,
) {
    let paths = (0..filenames.count())
        .map(|index| {
            // SAFETY: the index is generated from this NSArray's exact count
            // and AppKit keeps the array alive for the callback.
            let filename = unsafe { filenames.objectAtIndex(index) };
            PathBuf::from(filename.to_string())
        })
        .collect::<Vec<_>>();
    if !paths.is_empty() {
        deliver(ApplicationActivation::OpenFiles(
            DocumentActivation::from_paths(paths),
        ));
    }
}

extern "C" fn application_should_handle_reopen(
    _this: &AnyObject,
    _cmd: Sel,
    _application: &NSApplication,
    _has_visible_windows: Bool,
) -> Bool {
    deliver(ApplicationActivation::Reopen);
    Bool::YES
}

/// A subclass of `delegate`'s class that adds the activation callbacks.
///
/// The event loop (winit) owns the application delegate and panics if it is
/// replaced, so activation extends that delegate instead of installing its own.
fn activation_subclass(superclass: &AnyClass) -> &'static AnyClass {
    let name = format!("IncularActivation_{}", superclass.name());
    let Some(mut builder) = ClassBuilder::new(&name, superclass) else {
        return AnyClass::get(&name).expect("activation subclass was registered under its name");
    };
    // SAFETY: each signature matches the NSApplicationDelegate method of the
    // same selector: object arguments as references, BOOL as `Bool`.
    unsafe {
        builder.add_method(
            sel!(application:openURLs:),
            application_open_urls as extern "C" fn(_, _, _, _),
        );
        builder.add_method(
            sel!(application:openFiles:),
            application_open_files as extern "C" fn(_, _, _, _),
        );
        builder.add_method(
            sel!(applicationShouldHandleReopen:hasVisibleWindows:),
            application_should_handle_reopen as extern "C" fn(_, _, _, _) -> _,
        );
    }
    builder.register()
}

fn normalize_urls(urls: &NSArray<NSURL>) -> Vec<ApplicationActivation> {
    enum Batch {
        Files(Vec<PathBuf>),
        Urls(Vec<Url>),
    }

    fn flush(batch: Batch, activations: &mut Vec<ApplicationActivation>) {
        match batch {
            Batch::Files(paths) if !paths.is_empty() => activations.push(
                ApplicationActivation::OpenFiles(DocumentActivation::from_paths(paths)),
            ),
            Batch::Urls(urls) if !urls.is_empty() => {
                activations.push(ApplicationActivation::OpenUrls(UrlActivation::new(urls)))
            }
            Batch::Files(_) | Batch::Urls(_) => {}
        }
    }

    let mut activations = Vec::new();
    let mut batch: Option<Batch> = None;
    for index in 0..urls.count() {
        // SAFETY: index is bounded by this NSArray's count and the callback
        // retains the array for the duration of normalization.
        let native = unsafe { urls.objectAtIndex(index) };
        // SAFETY: NSURL accessors return retained Foundation values and do not
        // mutate AppKit state. This callback runs on the AppKit main thread.
        let item = unsafe {
            if native.isFileURL() {
                native
                    .path()
                    .map(|path| EitherActivation::File(PathBuf::from(path.to_string())))
            } else {
                native.absoluteString().and_then(|value| {
                    Url::parse(&value.to_string())
                        .ok()
                        .map(EitherActivation::Url)
                })
            }
        };
        let Some(item) = item else {
            continue;
        };
        match (batch.take(), item) {
            (Some(Batch::Files(mut paths)), EitherActivation::File(path)) => {
                paths.push(path);
                batch = Some(Batch::Files(paths));
            }
            (Some(Batch::Urls(mut parsed)), EitherActivation::Url(url)) => {
                parsed.push(url);
                batch = Some(Batch::Urls(parsed));
            }
            (Some(previous), EitherActivation::File(path)) => {
                flush(previous, &mut activations);
                batch = Some(Batch::Files(vec![path]));
            }
            (Some(previous), EitherActivation::Url(url)) => {
                flush(previous, &mut activations);
                batch = Some(Batch::Urls(vec![url]));
            }
            (None, EitherActivation::File(path)) => batch = Some(Batch::Files(vec![path])),
            (None, EitherActivation::Url(url)) => batch = Some(Batch::Urls(vec![url])),
        }
    }
    if let Some(batch) = batch {
        flush(batch, &mut activations);
    }
    activations
}

enum EitherActivation {
    File(PathBuf),
    Url(Url),
}

#[derive(Clone, Default)]
pub(crate) struct MacosActivationState {
    inner: Rc<ActivationInner>,
}

#[derive(Default)]
struct ActivationInner {
    watching: Cell<bool>,
}

impl MacosActivationState {
    pub(crate) fn start_watch(&self, deliver: Deliver) {
        if self.inner.watching.replace(true) {
            return;
        }
        let mtm = MainThreadMarker::new()
            .expect("AppKit application activation watch must be installed on the main thread");
        DELIVER.set(Some(deliver));
        let app = NSApplication::sharedApplication(mtm);
        // SAFETY: reads the application's delegate on the main thread.
        let Some(delegate) = (unsafe { app.delegate() }) else {
            // Without an event loop delegate AppKit has nowhere to send events.
            return;
        };
        // SAFETY: `ProtocolObject` is a `repr(C)` wrapper around `AnyObject`.
        let object: &AnyObject = unsafe { &*(&raw const *delegate).cast::<AnyObject>() };
        let class = object.class();
        if !class.name().starts_with("IncularActivation_") {
            // SAFETY: the subclass adds methods only (no ivars, so the
            // instance size is unchanged) and overrides nothing the delegate
            // already implements with a different signature.
            unsafe { AnyObject::set_class(object, activation_subclass(class)) };
        }
        // Setting the same delegate again makes AppKit re-check which
        // optional methods it responds to.
        app.setDelegate(Some(&*delegate));
    }
}

impl Drop for ActivationInner {
    fn drop(&mut self) {
        if self.watching.get() {
            DELIVER.set(None);
        }
    }
}
