#![cfg(all(debug_assertions, not(target_arch = "wasm32")))]

use incular_platform::{BrowserContextMenu, FlutterPlatformMessenger};
use std::{
    cell::Cell,
    future::{Future, ready},
    panic::{AssertUnwindSafe, catch_unwind},
    pin::pin,
    rc::Rc,
    task::{Context, Waker},
};

struct Host(Rc<Cell<usize>>);

impl FlutterPlatformMessenger for Host {
    type Error = std::convert::Infallible;

    fn send_platform_message(
        &mut self,
        _: &'static str,
        _: Vec<u8>,
    ) -> impl Future<Output = Result<Option<Vec<u8>>, Self::Error>> {
        self.0.set(self.0.get() + 1);
        ready(Ok(None))
    }
}

fn assert_non_web_panics<T>(future: impl Future<Output = T>) {
    let failure = catch_unwind(AssertUnwindSafe(|| {
        let _ = pin!(future).poll(&mut Context::from_waker(Waker::noop()));
    }))
    .expect_err("context menu methods assert outside the browser");
    let message = failure
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| failure.downcast_ref::<&str>().copied())
        .unwrap();
    assert!(message.contains("This has no effect on platforms other than web."));
}

#[test]
fn disabling_browser_context_menu_asserts_on_native_before_transport() {
    let calls = Rc::new(Cell::new(0));
    let mut menu = BrowserContextMenu::new(Host(calls.clone()));
    assert_non_web_panics(menu.disable_context_menu());
    assert_eq!(calls.get(), 0);
    assert!(menu.enabled());
}

#[test]
fn enabling_browser_context_menu_asserts_on_native_before_transport() {
    let calls = Rc::new(Cell::new(0));
    let mut menu = BrowserContextMenu::new(Host(calls.clone()));
    assert_non_web_panics(menu.enable_context_menu());
    assert_eq!(calls.get(), 0);
    assert!(menu.enabled());
}
