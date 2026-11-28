//! Optional ingress for engines that send separate key-data and raw streams.

use incular_core::KeyboardEvent;
use incular_widgets::RawKeyEvent;

use crate::{KeyMessage, raw_keyboard::LegacyRawPacketFilter};

#[derive(Clone, Debug, PartialEq)]
pub struct FlutterEngineKeyEvent {
    pub event: KeyboardEvent,
    pub synthesized: bool,
}

impl FlutterEngineKeyEvent {
    #[must_use]
    pub fn new(event: KeyboardEvent, synthesized: bool) -> Self {
        Self { event, synthesized }
    }
}

/// An empty packet establishes dual-stream mode without creating a key.
#[derive(Clone, Debug, PartialEq)]
pub enum FlutterEngineKeyData {
    Empty,
    Event(FlutterEngineKeyEvent),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FlutterKeyTransitMode {
    RawOnly,
    KeyDataThenRaw,
}

/// The optional Flutter engine compatibility boundary. Native hosts with
/// already complete key messages continue to call Runtime directly.
#[derive(Default)]
pub struct FlutterKeyTransit {
    mode: Option<FlutterKeyTransitMode>,
    pending: Vec<FlutterEngineKeyEvent>,
    filter: LegacyRawPacketFilter,
}

impl FlutterKeyTransit {
    #[must_use]
    pub fn mode(&self) -> Option<FlutterKeyTransitMode> {
        self.mode
    }

    /// A solitary synthesized event goes straight to the focused listeners.
    /// Otherwise the next raw packet supplies the batch's routing boundary.
    /// The typed bridge itself never consumes an engine key-data packet.
    pub fn handle_key_data(
        &mut self,
        data: FlutterEngineKeyData,
        deliver: impl FnOnce(KeyMessage) -> bool,
    ) -> bool {
        let mode = self
            .mode
            .get_or_insert(FlutterKeyTransitMode::KeyDataThenRaw);
        debug_assert_eq!(*mode, FlutterKeyTransitMode::KeyDataThenRaw);
        if *mode == FlutterKeyTransitMode::RawOnly {
            return false;
        }
        if let FlutterEngineKeyData::Event(key) = data {
            if key.synthesized && self.pending.is_empty() {
                deliver(KeyMessage::FlutterBatch {
                    typed: vec![key],
                    raw: None,
                });
            } else {
                self.pending.push(key);
            }
        }
        false
    }

    pub fn handle_raw_key(
        &mut self,
        raw: RawKeyEvent,
        deliver: impl FnOnce(KeyMessage) -> bool,
    ) -> bool {
        let mode = *self.mode.get_or_insert(FlutterKeyTransitMode::RawOnly);
        // Non-dispatchable IME packets establish mode but do not drain
        // pending typed input: the next deliverable raw packet does.
        if !self.filter.admits(&raw) {
            return true;
        }
        match mode {
            FlutterKeyTransitMode::RawOnly => deliver(KeyMessage::Raw(raw)),
            FlutterKeyTransitMode::KeyDataThenRaw => deliver(KeyMessage::FlutterBatch {
                typed: std::mem::take(&mut self.pending),
                raw: Some(raw),
            }),
        }
    }
}
