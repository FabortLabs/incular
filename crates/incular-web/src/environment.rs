//! Page-level preferences and lifecycle.
//!
//! Winit already reports the color scheme per window. This module reads the
//! application-scoped preferences a page exposes (languages, reduced motion,
//! contrast) and maps document visibility to application activity.

use incular_platform::{
    PlatformLifecycle, SystemEnvironmentPreferences, canonicalize_system_locales,
};
use std::{cell::RefCell, rc::Rc, sync::Arc};
use wasm_bindgen::{JsCast as _, closure::Closure};

const REDUCED_MOTION: &str = "(prefers-reduced-motion: reduce)";
const HIGH_CONTRAST: &str = "(prefers-contrast: more), (forced-colors: active)";

#[derive(Default)]
struct WatchState {
    changed: bool,
    lifecycle: Vec<PlatformLifecycle>,
}

#[derive(Default)]
pub(crate) struct WebEnvironment {
    state: Rc<RefCell<WatchState>>,
}

impl WebEnvironment {
    pub(crate) fn preferences(&self) -> SystemEnvironmentPreferences {
        let window = web_sys::window();
        let locales = window.as_ref().map(|window| {
            canonicalize_system_locales(
                window
                    .navigator()
                    .languages()
                    .iter()
                    .filter_map(|language| language.as_string()),
            )
        });
        SystemEnvironmentPreferences {
            locales: locales.filter(|locales| !locales.is_empty()),
            reduced_motion: media_matches(REDUCED_MOTION),
            high_contrast: media_matches(HIGH_CONTRAST),
            ..SystemEnvironmentPreferences::default()
        }
    }

    pub(crate) fn page_visible(&self) -> Option<bool> {
        let document = web_sys::window()?.document()?;
        Some(document.visibility_state() == web_sys::VisibilityState::Visible)
    }

    /// Installs page listeners for the application's lifetime. The page
    /// keeps them until unload, so their closures are intentionally leaked.
    pub(crate) fn start_watch(&self, wake: Arc<dyn Fn() + Send + Sync>) {
        let Some(window) = web_sys::window() else {
            return;
        };
        for query in [REDUCED_MOTION, HIGH_CONTRAST] {
            if let Ok(Some(list)) = window.match_media(query) {
                listen(&list, "change", &self.state, &wake, |state| {
                    state.changed = true;
                });
            }
        }
        listen(&window, "languagechange", &self.state, &wake, |state| {
            state.changed = true;
        });
        if let Some(document) = window.document() {
            let observed = document.clone();
            listen(
                &document,
                "visibilitychange",
                &self.state,
                &wake,
                // A hidden page may be discarded or closed without another
                // event, so it is suspended (which flushes restoration), not
                // merely inactive.
                move |state| {
                    state.lifecycle.push(
                        if observed.visibility_state() == web_sys::VisibilityState::Visible {
                            PlatformLifecycle::Resumed
                        } else {
                            PlatformLifecycle::Suspended
                        },
                    );
                },
            );
        }
    }

    pub(crate) fn take_change(&self) -> bool {
        std::mem::take(&mut self.state.borrow_mut().changed)
    }

    pub(crate) fn take_lifecycle_events(&self) -> Vec<PlatformLifecycle> {
        std::mem::take(&mut self.state.borrow_mut().lifecycle)
    }
}

fn media_matches(query: &str) -> Option<bool> {
    Some(web_sys::window()?.match_media(query).ok()??.matches())
}

fn listen(
    target: &web_sys::EventTarget,
    event: &str,
    state: &Rc<RefCell<WatchState>>,
    wake: &Arc<dyn Fn() + Send + Sync>,
    record: impl Fn(&mut WatchState) + 'static,
) {
    let state = state.clone();
    let wake = wake.clone();
    let callback = Closure::<dyn FnMut()>::new(move || {
        record(&mut state.borrow_mut());
        wake();
    });
    if target
        .add_event_listener_with_callback(event, callback.as_ref().unchecked_ref())
        .is_ok()
    {
        callback.forget();
    }
}
