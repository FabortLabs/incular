use incular_platform::{
    BrowserContextMenu, DomContextMenuMessenger, FlutterPlatformMessenger,
    message_codecs::{JsonMethodCodec, MethodCall},
};
use serde_json::Value;
use std::{
    cell::RefCell,
    future::Future,
    pin::pin,
    rc::Rc,
    task::{Context, Poll, Waker},
};
use wasm_bindgen::prelude::*;
use web_sys::EventTarget;

type WireLog = Rc<RefCell<Vec<(&'static str, MethodCall<Value>)>>>;
struct RecordingHost {
    dom: DomContextMenuMessenger,
    wire: WireLog,
}

impl FlutterPlatformMessenger for RecordingHost {
    type Error = String;

    async fn send_platform_message(
        &mut self,
        channel: &'static str,
        payload: Vec<u8>,
    ) -> Result<Option<Vec<u8>>, Self::Error> {
        let call = JsonMethodCodec::decode_method_call(&payload).map_err(|e| e.to_string())?;
        self.wire.borrow_mut().push((channel, call));
        self.dom
            .send_platform_message(channel, payload)
            .await
            .map_err(|e| e.to_string())
    }
}

#[wasm_bindgen]
pub struct ContextMenuProbe {
    menu: BrowserContextMenu<RecordingHost>,
    wire: WireLog,
}

#[wasm_bindgen]
impl ContextMenuProbe {
    #[wasm_bindgen(constructor)]
    pub fn new(element: EventTarget) -> ContextMenuProbe {
        let wire = WireLog::default();
        Self {
            menu: BrowserContextMenu::new(RecordingHost {
                dom: DomContextMenuMessenger::new(element),
                wire: wire.clone(),
            }),
            wire,
        }
    }

    pub fn enabled(&self) -> bool {
        self.menu.enabled()
    }

    pub fn set_enabled(&mut self, enabled: bool) -> Result<(), JsValue> {
        fn wait<T>(future: impl Future<Output = T>) -> Result<T, JsValue> {
            match pin!(future).poll(&mut Context::from_waker(Waker::noop())) {
                Poll::Ready(value) => Ok(value),
                Poll::Pending => Err(JsValue::from("DOM messenger should reply immediately")),
            }
        }

        let result = if enabled {
            wait(self.menu.enable_context_menu())?
        } else {
            wait(self.menu.disable_context_menu())?
        };
        result.map_err(|e| JsValue::from(e.to_string()))?;
        let calls = self.wire.take();
        let expected = if enabled {
            "enableContextMenu"
        } else {
            "disableContextMenu"
        };
        if calls
            != [(
                "flutter/contextmenu",
                MethodCall {
                    method: expected.to_owned(),
                    arguments: Value::Null,
                },
            )]
        {
            return Err(JsValue::from(format!("wrong channel request: {calls:?}")));
        }
        if self.menu.enabled() != enabled {
            return Err(JsValue::from("completed call did not update state"));
        }
        Ok(())
    }
}
