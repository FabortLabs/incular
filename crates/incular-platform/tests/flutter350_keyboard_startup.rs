use std::{
    future::Future,
    pin::pin,
    task::{Context, Poll, Waker},
};

use incular_core::{KeyState, LogicalKeyboardKey, PhysicalKeyboardKey};
use incular_platform::{
    FlutterKeyboardState, FlutterKeyboardSyncError, FlutterPlatformMessenger,
    message_codecs::{MessageValue as V, MethodCall, StandardMethodCodec as Codec},
};

struct Engine {
    sent: Vec<(&'static str, MethodCall<V>)>,
    result: Option<V>,
}

impl Engine {
    fn replying(result: Option<V>) -> Self {
        Self {
            sent: Vec::new(),
            result,
        }
    }
}

impl FlutterPlatformMessenger for Engine {
    type Error = std::convert::Infallible;

    async fn send_platform_message(
        &mut self,
        channel: &'static str,
        message: Option<Vec<u8>>,
    ) -> Result<Option<Vec<u8>>, Self::Error> {
        self.sent.push((
            channel,
            Codec::decode_method_call(&message.unwrap()).unwrap(),
        ));
        Ok(self
            .result
            .as_ref()
            .map(|value| Codec::encode_success_envelope(value).unwrap()))
    }
}

fn wait<T>(future: impl Future<Output = T>) -> T {
    match pin!(future).poll(&mut Context::from_waker(Waker::noop())) {
        Poll::Ready(result) => result,
        Poll::Pending => panic!("the mock engine replies immediately"),
    }
}

#[test]
fn startup_uses_the_binary_keyboard_channel_and_preserves_both_exact_key_ids() {
    let mut engine = Engine::replying(Some(V::Map(vec![(V::Int(1), V::Int(1))])));
    let mut keyboard = wait(FlutterKeyboardState::start(&mut engine)).unwrap();
    assert_eq!(engine.sent.len(), 1);
    let (channel, call) = &engine.sent[0];
    assert_eq!(*channel, "flutter/keyboard");
    assert_eq!(call.method, "getKeyboardState");
    assert_eq!(call.arguments, V::Null);
    let physical = PhysicalKeyboardKey::new(1);
    let logical = LogicalKeyboardKey::new(1);
    assert_eq!(keyboard.physical_keys_pressed(), [physical].into());
    assert_eq!(keyboard.logical_keys_pressed(), [logical].into());
    assert_eq!(keyboard.look_up_layout(physical), Some(logical));
    keyboard.observe(physical, LogicalKeyboardKey::new(2), KeyState::Up);
    assert!(keyboard.physical_keys_pressed().is_empty());
    assert!(keyboard.logical_keys_pressed().is_empty());
}

#[test]
fn synchronization_merges_valid_pairs_and_leaves_state_unchanged_on_invalid_or_missing_reply() {
    let mut keyboard = FlutterKeyboardState::default();
    let physical = |n| PhysicalKeyboardKey::new(n);
    let logical = |n| LogicalKeyboardKey::new(n);
    keyboard.observe(physical(1), logical(2), KeyState::Down);
    keyboard.observe(physical(2), logical(3), KeyState::Down);
    wait(keyboard.sync(&mut Engine::replying(Some(V::Map(vec![
        (V::Int(2), V::Int(0x11_1234_5678)),
        (V::Int(3), V::Int(5)),
    ])))))
    .unwrap();
    assert_eq!(keyboard.look_up_layout(physical(1)), Some(logical(2)));
    assert_eq!(
        keyboard.look_up_layout(physical(2)),
        Some(logical(0x11_1234_5678))
    );
    assert_eq!(keyboard.look_up_layout(physical(3)), Some(logical(5)));

    let broken = V::Map(vec![(V::Int(1), V::Int(9)), (V::Int(-1), V::Int(8))]);
    assert!(matches!(
        wait(keyboard.sync(&mut Engine::replying(Some(broken)))),
        Err(FlutterKeyboardSyncError::InvalidKeyMap)
    ));
    for absent in [None, Some(V::Null)] {
        wait(keyboard.sync(&mut Engine::replying(absent))).unwrap();
    }
    assert_eq!(keyboard.look_up_layout(physical(1)), Some(logical(2)));
    assert_eq!(keyboard.physical_keys_pressed().len(), 3);
}
