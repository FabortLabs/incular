//! Browser session history as router route information.

use std::fmt;

use incular_widgets::{
    MemoryRouteInformationProvider, RouteInformation, RouteInformationListener,
    RouteInformationProvider, RouteInformationReportingType, RouteInformationSubscription,
    RouterError,
};

/// How a route location is written into the page URL.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum UrlStrategy {
    /// The route lives in the fragment (`/index.html#/settings`). Works on
    /// any static file host, because the server only ever sees the page.
    #[default]
    Hash,
    /// The route is the URL path (`/settings`). The server must answer every
    /// route path with the application page, and the application must be
    /// served from the site root.
    Path,
}

/// A [`RouteInformationProvider`] backed by the page's session history.
///
/// The router starts at the route in the page URL, so links and reloads open
/// the route they name. Routes the application navigates to become history
/// entries, and the browser's back and forward buttons deliver the entry
/// they move to as a platform route. Route state is stored in the history
/// entry as JSON.
///
/// Off the browser target this behaves like a
/// [`MemoryRouteInformationProvider`] starting at `/`, so the same
/// application code builds for native windows.
#[derive(Clone)]
pub struct BrowserRouteInformationProvider {
    routes: MemoryRouteInformationProvider,
    strategy: UrlStrategy,
    #[cfg(target_arch = "wasm32")]
    _history: std::rc::Rc<page::HistoryListener>,
}

impl BrowserRouteInformationProvider {
    /// Keeps routes in the URL fragment.
    #[must_use]
    pub fn new() -> Self {
        Self::with_strategy(UrlStrategy::Hash)
    }

    /// Keeps routes in the URL as `strategy` describes.
    #[must_use]
    pub fn with_strategy(strategy: UrlStrategy) -> Self {
        #[cfg(target_arch = "wasm32")]
        {
            let routes = MemoryRouteInformationProvider::new(page::read(strategy));
            Self {
                _history: page::HistoryListener::install(routes.clone(), strategy),
                routes,
                strategy,
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        Self {
            routes: MemoryRouteInformationProvider::root(),
            strategy,
        }
    }

    /// Returns how routes are written into the page URL.
    #[must_use]
    pub fn strategy(&self) -> UrlStrategy {
        self.strategy
    }
}

impl Default for BrowserRouteInformationProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for BrowserRouteInformationProvider {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("BrowserRouteInformationProvider")
            .field("value", &self.routes.value())
            .field("strategy", &self.strategy)
            .finish_non_exhaustive()
    }
}

impl RouteInformationProvider for BrowserRouteInformationProvider {
    fn value(&self) -> RouteInformation {
        self.routes.value()
    }

    fn set_platform_route_information(&self, information: RouteInformation) {
        self.routes.set_platform_route_information(information);
    }

    fn subscribe(&self, listener: RouteInformationListener) -> RouteInformationSubscription {
        self.routes.subscribe(listener)
    }

    fn router_reports_new_route_information(
        &self,
        information: RouteInformation,
        reporting: RouteInformationReportingType,
    ) -> Result<(), RouterError> {
        #[cfg(target_arch = "wasm32")]
        page::report(&information, reporting, self.strategy)?;
        self.routes
            .router_reports_new_route_information(information, reporting)
    }
}

#[cfg(target_arch = "wasm32")]
mod page {
    use incular_widgets::{
        MemoryRouteInformationProvider, RouteInformation, RouteInformationReportingType,
        RouterError,
    };
    use wasm_bindgen::{JsCast as _, JsValue, closure::Closure};

    use super::UrlStrategy;

    /// Delivers history traversal to the provider until dropped.
    pub(super) struct HistoryListener {
        callback: Closure<dyn FnMut(web_sys::Event)>,
    }

    impl HistoryListener {
        pub(super) fn install(
            routes: MemoryRouteInformationProvider,
            strategy: UrlStrategy,
        ) -> std::rc::Rc<Self> {
            let callback = Closure::<dyn FnMut(web_sys::Event)>::new(move |_| {
                routes.set_value(read(strategy));
                crate::wake_host();
            });
            if let Some(window) = web_sys::window() {
                // Traversal, and fragment edits in the address bar, both
                // arrive as `popstate`.
                let _ = window.add_event_listener_with_callback(
                    "popstate",
                    callback.as_ref().unchecked_ref(),
                );
            }
            std::rc::Rc::new(Self { callback })
        }
    }

    impl Drop for HistoryListener {
        fn drop(&mut self) {
            if let Some(window) = web_sys::window() {
                let _ = window.remove_event_listener_with_callback(
                    "popstate",
                    self.callback.as_ref().unchecked_ref(),
                );
            }
        }
    }

    /// Reads the route the page URL and current history entry name.
    pub(super) fn read(strategy: UrlStrategy) -> RouteInformation {
        let Some(window) = web_sys::window() else {
            return RouteInformation::default();
        };
        let state = window
            .history()
            .ok()
            .and_then(|history| history.state().ok())
            .and_then(|state| state_to_json(&state));
        RouteInformation::with_state(location(&window, strategy), state)
    }

    fn location(window: &web_sys::Window, strategy: UrlStrategy) -> String {
        let location = window.location();
        match strategy {
            UrlStrategy::Hash => location
                .hash()
                .unwrap_or_default()
                .trim_start_matches('#')
                .to_owned(),
            UrlStrategy::Path => format!(
                "{}{}{}",
                location.pathname().unwrap_or_default(),
                location.search().unwrap_or_default(),
                location.hash().unwrap_or_default()
            ),
        }
    }

    /// Writes a route the router chose into session history.
    pub(super) fn report(
        information: &RouteInformation,
        reporting: RouteInformationReportingType,
        strategy: UrlStrategy,
    ) -> Result<(), RouterError> {
        if reporting == RouteInformationReportingType::None {
            return Ok(());
        }
        let window = web_sys::window().ok_or_else(|| RouterError::Provider("no window".into()))?;
        let history = window
            .history()
            .map_err(|error| RouterError::Provider(format!("{error:?}")))?;
        let url = match strategy {
            UrlStrategy::Hash => format!("#{}", information.location()),
            UrlStrategy::Path => information.location().to_owned(),
        };
        let state = information
            .state()
            .and_then(json_to_state)
            .unwrap_or(JsValue::NULL);
        // The router also reports the route it was just given, after a
        // traversal or at startup; that must not add an entry.
        let current = RouteInformation::new(location(&window, strategy));
        let result = if reporting == RouteInformationReportingType::Navigate
            && current.location() != information.location()
        {
            history.push_state_with_url(&state, "", Some(&url))
        } else {
            history.replace_state_with_url(&state, "", Some(&url))
        };
        result.map_err(|error| RouterError::Provider(format!("{error:?}")))
    }

    fn state_to_json(state: &JsValue) -> Option<serde_json::Value> {
        if state.is_null() || state.is_undefined() {
            return None;
        }
        let text = js_sys::JSON::stringify(state).ok()?.as_string()?;
        serde_json::from_str(&text).ok()
    }

    fn json_to_state(value: &serde_json::Value) -> Option<JsValue> {
        js_sys::JSON::parse(&serde_json::to_string(value).ok()?).ok()
    }
}
