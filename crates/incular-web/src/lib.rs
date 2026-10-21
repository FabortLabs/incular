//! Browser adapter for Incular's shared Winit host.
//!
//! A browser page is one more Winit platform: `incular-desktop` owns the
//! event loop, input, renderer, accessibility and teardown paths here exactly
//! as it does on native targets. This crate only binds windows to page
//! canvases and reports page-level environment and lifecycle state. Native
//! services a page cannot offer (file dialogs, global shortcuts, trays,
//! single-instance ownership, native popups) are reported unsupported by the
//! shared host.
//!
//! Every entry point returns immediately on `wasm32`: the page owns the event
//! loop and the application keeps running in browser callbacks.

#[cfg(target_arch = "wasm32")]
mod clipboard;
#[cfg(target_arch = "wasm32")]
mod environment;
#[cfg(target_arch = "wasm32")]
mod page_events;

/// Where the first application window draws. Its `initial_logical_size`
/// and size limits do not apply: the page decides how large it is, and the
/// window follows the canvas's CSS size.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub enum CanvasTarget {
    /// A canvas appended to the document body that fills the viewport.
    #[default]
    Page,
    /// Draw into the existing `<canvas>` element with this `id`. Page CSS
    /// sizes it.
    Element(String),
}

/// Failure to start the browser host.
#[derive(Debug, thiserror::Error)]
pub enum RunError {
    #[error(transparent)]
    Host(#[from] incular_desktop::RunError),
    #[error("no element with id `{0}` exists in the document")]
    CanvasNotFound(String),
    #[error("element `{0}` is not a <canvas>")]
    NotACanvas(String),
    #[error("the browser host requires a wasm32 target")]
    UnsupportedTarget,
}

/// Runs an application whose first window fills the page.
pub fn run_application(application: incular_runtime::Application) -> Result<(), RunError> {
    run_application_in(application, CanvasTarget::Page)
}

/// Runs an application whose first window draws into `canvas`. Later
/// windows keep their logical size in canvases floating over the page.
pub fn run_application_in(
    application: incular_runtime::Application,
    canvas: CanvasTarget,
) -> Result<(), RunError> {
    #[cfg(target_arch = "wasm32")]
    {
        let page_owned = matches!(canvas, CanvasTarget::Element(_));
        let services = WebPlatformServices::new(resolve_canvas(canvas)?, page_owned);
        incular_desktop::run_application_with_services(application, services)?;
        Ok(())
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = (application, canvas);
        Err(RunError::UnsupportedTarget)
    }
}

/// Runs a standalone retained runtime in a canvas that fills the page.
pub fn run_window(
    runtime: incular_runtime::Runtime,
    on_action: impl FnMut(incular_widgets::internal::ActionId) + 'static,
) -> Result<(), RunError> {
    #[cfg(target_arch = "wasm32")]
    {
        let services = WebPlatformServices::new(resolve_canvas(CanvasTarget::Page)?, false);
        incular_desktop::run_window_with_services(runtime, on_action, services)?;
        Ok(())
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = (runtime, on_action);
        Err(RunError::UnsupportedTarget)
    }
}

/// Touches reach the application's gesture recognizers instead of panning
/// or zooming the page, and long presses select nothing.
#[cfg(target_arch = "wasm32")]
const INTERACTIVE_CANVAS_STYLE: &str = "touch-action:none;user-select:none;\
     -webkit-user-select:none;-webkit-tap-highlight-color:transparent;";

/// Fills the viewport without affecting the page's flow or scrolling.
#[cfg(target_arch = "wasm32")]
const PAGE_CANVAS_STYLE: &str =
    "display:block;position:fixed;inset:0;width:100%;height:100%;outline:none;";

/// Later windows float over the page in a cascade from the bottom-right
/// corner, away from where most layouts start. Winit adds their logical size
/// as inline `width`/`height`.
#[cfg(target_arch = "wasm32")]
fn floating_canvas_style(index: u32) -> String {
    let offset = 24 + 32 * (index % 8);
    format!(
        "display:block;position:fixed;right:{offset}px;bottom:{offset}px;outline:none;\
         box-shadow:0 8px 32px rgba(0,0,0,0.45);"
    )
}

#[cfg(target_arch = "wasm32")]
fn document() -> Option<web_sys::Document> {
    web_sys::window().and_then(|window| window.document())
}

/// Creates a styled canvas appended to the document body.
#[cfg(target_arch = "wasm32")]
fn append_canvas(style: &str) -> Option<web_sys::HtmlCanvasElement> {
    use wasm_bindgen::JsCast as _;

    let document = document()?;
    let canvas = document
        .create_element("canvas")
        .ok()?
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .ok()?;
    canvas
        .set_attribute("style", &format!("{style}{INTERACTIVE_CANVAS_STYLE}"))
        .ok()?;
    document.body()?.append_child(&canvas).ok()?;
    Some(canvas)
}

#[cfg(target_arch = "wasm32")]
fn resolve_canvas(target: CanvasTarget) -> Result<Option<web_sys::HtmlCanvasElement>, RunError> {
    use wasm_bindgen::JsCast as _;

    let id = match target {
        // Without a document body, Winit appends a plain canvas itself.
        CanvasTarget::Page => return Ok(append_canvas(PAGE_CANVAS_STYLE)),
        CanvasTarget::Element(id) => id,
    };
    let element = document()
        .and_then(|document| document.get_element_by_id(&id))
        .ok_or_else(|| RunError::CanvasNotFound(id.clone()))?;
    let canvas = element
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .map_err(|_| RunError::NotACanvas(id))?;
    // Page CSS owns layout, but touch panning would steal every gesture.
    let _ = canvas.style().set_property("touch-action", "none");
    Ok(Some(canvas))
}

/// Registers the bundled Inter faces with every text engine on this thread.
/// Engines pick them up at their next layout, so text measured before launch
/// is re-laid out with them.
#[cfg(all(target_arch = "wasm32", feature = "default-font"))]
fn register_default_font() {
    incular_text::register_application_font(include_bytes!("../fonts/Inter-Regular.ttf"));
    incular_text::register_application_font(include_bytes!("../fonts/Inter-Bold.ttf"));
}

#[cfg(target_arch = "wasm32")]
struct WebPlatformServices {
    /// Taken by the first application window.
    canvas: std::cell::Cell<Option<web_sys::HtmlCanvasElement>>,
    /// The first window's canvas; its title is the document title.
    primary_canvas: Option<web_sys::HtmlCanvasElement>,
    /// A canvas the page provided, which outlives its window.
    page_canvas: Option<web_sys::HtmlCanvasElement>,
    /// Application windows created so far; the first one is page-sized.
    windows: std::cell::Cell<u32>,
    environment: environment::WebEnvironment,
}

#[cfg(target_arch = "wasm32")]
impl WebPlatformServices {
    fn new(canvas: Option<web_sys::HtmlCanvasElement>, page_owned: bool) -> Self {
        // `eprintln!` and panics are otherwise invisible in a page.
        console_error_panic_hook::set_once();
        #[cfg(feature = "default-font")]
        register_default_font();
        page_events::install();
        clipboard::install();
        Self {
            primary_canvas: canvas.clone(),
            page_canvas: canvas.clone().filter(|_| page_owned),
            canvas: std::cell::Cell::new(canvas),
            windows: std::cell::Cell::new(0),
            environment: environment::WebEnvironment::default(),
        }
    }

    fn is_primary(&self, window: &winit::window::Window) -> bool {
        use winit::platform::web::WindowExtWebSys as _;

        self.primary_canvas.is_some() && window.canvas() == self.primary_canvas
    }
}

#[cfg(target_arch = "wasm32")]
impl incular_desktop::DesktopPlatformServices for WebPlatformServices {
    fn application_shell(&self) -> &dyn incular_desktop::DesktopApplicationShellServices {
        self
    }

    fn configure_window_attributes(
        &self,
        attributes: winit::window::WindowAttributes,
    ) -> winit::window::WindowAttributes {
        use winit::platform::web::WindowAttributesExtWebSys as _;

        let index = self.windows.get();
        self.windows.set(index + 1);
        if index == 0 {
            // The page sizes the first window; a requested size would pin
            // the canvas with inline CSS and stop it following its host.
            let mut attributes = attributes;
            attributes.inner_size = None;
            attributes.min_inner_size = None;
            attributes.max_inner_size = None;
            return match self.canvas.take() {
                Some(canvas) => attributes.with_canvas(Some(canvas)),
                None => attributes.with_append(true),
            };
        }
        match append_canvas(&floating_canvas_style(index - 1)) {
            Some(canvas) => attributes.with_canvas(Some(canvas)),
            None => attributes.with_append(true),
        }
    }

    fn keyboard_event_modifiers(&self) -> Option<winit::keyboard::ModifiersState> {
        page_events::current_modifiers()
    }

    fn window_clipboard(&self) -> Option<Box<dyn incular_platform::Clipboard>> {
        Some(Box::new(clipboard::PageClipboard))
    }

    fn window_created(&self, window: &winit::window::Window, title: &str) {
        if self.is_primary(window) {
            set_document_title(title);
            // Keyboard input reaches the application without a first click.
            window.focus_window();
        }
    }

    fn window_title_changed(&self, window: &winit::window::Window, title: &str) {
        if self.is_primary(window) {
            set_document_title(title);
        }
    }

    fn window_destroyed(&self, window: &winit::window::Window) {
        use winit::platform::web::WindowExtWebSys as _;

        // Winit leaves a closed window's canvas, and its last frame, in the
        // document.
        if let Some(canvas) = window
            .canvas()
            .filter(|canvas| Some(canvas) != self.page_canvas.as_ref())
        {
            canvas.remove();
        }
    }

    fn system_environment_preferences(&self) -> incular_platform::SystemEnvironmentPreferences {
        self.environment.preferences()
    }

    fn application_active(&self) -> Option<bool> {
        self.environment.page_visible()
    }

    fn start_system_environment_watch(&self, wake: std::sync::Arc<dyn Fn() + Send + Sync>) {
        self.environment.start_watch(wake);
    }

    fn take_system_environment_change(&self) -> bool {
        self.environment.take_change()
    }

    fn take_application_lifecycle_events(&self) -> Vec<incular_platform::PlatformLifecycle> {
        self.environment.take_lifecycle_events()
    }
}

#[cfg(target_arch = "wasm32")]
fn set_document_title(title: &str) {
    if let Some(document) = document().filter(|_| !title.is_empty()) {
        document.set_title(title);
    }
}

/// Trays, notifications and taskbar state have no page equivalent; the
/// default implementation reports each one unsupported.
#[cfg(target_arch = "wasm32")]
impl incular_desktop::DesktopApplicationShellServices for WebPlatformServices {}
