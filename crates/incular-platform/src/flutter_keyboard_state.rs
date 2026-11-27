//! Startup and resynchronization for an embedder using Flutter key IDs.

use std::collections::{BTreeMap, BTreeSet};

use incular_core::{KeyState, LogicalKeyboardKey, PhysicalKeyboardKey};

use crate::{
    FlutterPlatformMessenger,
    message_codecs::{MessageValue, MethodCall, MethodCodecError, StandardMethodCodec},
};

#[derive(Debug, thiserror::Error)]
pub enum FlutterKeyboardSyncError<E> {
    #[error(transparent)]
    Codec(#[from] MethodCodecError),
    #[error("while requesting the platform's pressed keys: {0}")]
    Transport(E),
    #[error("keyboard state requires a map of physical HID usages to logical key IDs")]
    InvalidKeyMap,
}

/// Tracks pressed key pairs when the host uses the Flutter keyboard wire
/// protocol. This does not replace the ordinary native key-code path.
#[derive(Clone, Debug, Default)]
pub struct FlutterKeyboardState {
    pressed: BTreeMap<PhysicalKeyboardKey, LogicalKeyboardKey>,
}

impl FlutterKeyboardState {
    pub const CHANNEL: &str = "flutter/keyboard";

    /// Reads the engine state before routing key events, including keys
    /// already held before the view started.
    pub async fn start<M: FlutterPlatformMessenger>(
        messenger: &mut M,
    ) -> Result<Self, FlutterKeyboardSyncError<M::Error>> {
        let mut state = Self::default();
        state.sync(messenger).await?;
        Ok(state)
    }

    /// Merges the platform's reported presses into the current state.
    /// Missing and null replies leave state unchanged. A malformed map
    /// cannot leave a partially imported state.
    pub async fn sync<M: FlutterPlatformMessenger>(
        &mut self,
        messenger: &mut M,
    ) -> Result<(), FlutterKeyboardSyncError<M::Error>> {
        let payload = StandardMethodCodec::encode_method_call(&MethodCall {
            method: "getKeyboardState".into(),
            arguments: MessageValue::Null,
        })?;
        let Some(reply) = messenger
            .send_platform_message(Self::CHANNEL, Some(payload))
            .await
            .map_err(FlutterKeyboardSyncError::Transport)?
        else {
            return Ok(());
        };
        let entries = match StandardMethodCodec::decode_envelope(&reply)? {
            MessageValue::Null => return Ok(()),
            MessageValue::Map(entries) => entries,
            _ => return Err(FlutterKeyboardSyncError::InvalidKeyMap),
        };
        let imported = entries
            .into_iter()
            .map(|(physical, logical)| {
                let (MessageValue::Int(physical), MessageValue::Int(logical)) = (physical, logical)
                else {
                    return Err(FlutterKeyboardSyncError::InvalidKeyMap);
                };
                let physical =
                    u32::try_from(physical).map_err(|_| FlutterKeyboardSyncError::InvalidKeyMap)?;
                let logical =
                    u64::try_from(logical).map_err(|_| FlutterKeyboardSyncError::InvalidKeyMap)?;
                Ok((
                    PhysicalKeyboardKey::new(physical),
                    LogicalKeyboardKey::new(logical),
                ))
            })
            .collect::<Result<Vec<_>, FlutterKeyboardSyncError<M::Error>>>()?;
        self.pressed.extend(imported);
        Ok(())
    }

    /// Retains the physical-to-logical association used by subsequent
    /// events. Releases clear the physical press even after a layout change.
    pub fn observe(
        &mut self,
        physical: PhysicalKeyboardKey,
        logical: LogicalKeyboardKey,
        state: KeyState,
    ) {
        match state {
            KeyState::Down => {
                self.pressed.insert(physical, logical);
            }
            KeyState::Up => {
                self.pressed.remove(&physical);
            }
        }
    }

    #[must_use]
    pub fn look_up_layout(&self, key: PhysicalKeyboardKey) -> Option<LogicalKeyboardKey> {
        self.pressed.get(&key).copied()
    }

    #[must_use]
    pub fn physical_keys_pressed(&self) -> BTreeSet<PhysicalKeyboardKey> {
        self.pressed.keys().copied().collect()
    }

    #[must_use]
    pub fn logical_keys_pressed(&self) -> BTreeSet<LogicalKeyboardKey> {
        self.pressed.values().copied().collect()
    }
}
