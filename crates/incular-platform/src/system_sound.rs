//! Portable requests for Flutter's short system sounds.

use std::{cell::RefCell, rc::Rc};

use crate::flutter_platform_channel::{
    FlutterPlatformChannelError, FlutterPlatformMessenger, invoke_optional_json,
};

/// A sound provided by the operating system, not by the application.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SystemSoundType {
    /// Short feedback for a key or button press.
    Click,
    /// Short feedback for a picker value change.
    Tick,
    /// An alert that calls attention to a rejected action.
    Alert,
}

type SoundHandler = Rc<dyn Fn(SystemSoundType)>;

/// Window-bound system sound service. An unsupported or unbound host
/// ignores requests; the service never substitutes an application sound.
#[derive(Clone, Default)]
pub struct SystemSound {
    handler: Rc<RefCell<Option<SoundHandler>>>,
}

impl SystemSound {
    /// Requests the native sound if the current host provides it.
    pub fn play(&self, sound: SystemSoundType) {
        let handler = self.handler.borrow().clone();
        if let Some(handler) = handler {
            handler(sound);
        }
    }

    /// Plays a sound through a Flutter-compatible embedder. The native
    /// typed service remains available through [Self::play].
    pub async fn play_on_flutter_channel<M: FlutterPlatformMessenger>(
        messenger: &mut M,
        sound: SystemSoundType,
    ) -> Result<(), FlutterPlatformChannelError<M::Error>> {
        let name = match sound {
            SystemSoundType::Click => "SystemSoundType.click",
            SystemSoundType::Alert => "SystemSoundType.alert",
            SystemSoundType::Tick => "SystemSoundType.tick",
        };
        invoke_optional_json(
            messenger,
            "flutter/platform",
            "SystemSound.play",
            serde_json::Value::String(name.to_owned()),
        )
        .await
    }

    /// Binds the window host's native implementation.
    #[doc(hidden)]
    pub fn bind_host(&self, handler: impl Fn(SystemSoundType) + 'static) {
        drop(self.handler.replace(Some(Rc::new(handler))));
    }

    /// Unbinds the host, including for service handles already in callbacks.
    #[doc(hidden)]
    pub fn unbind_host(&self) {
        drop(self.handler.replace(None));
    }
}
