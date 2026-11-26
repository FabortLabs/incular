//! Flutter-compatible lifecycle state and host-message dispatch. This retains
//! the full view lifecycle without reinterpreting native process shutdown.

use std::{collections::BTreeMap, fmt, str::FromStr};

/// Application lifecycle in Flutter's state-machine order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum AppLifecycleState {
    Detached,
    Resumed,
    Inactive,
    Hidden,
    Paused,
}

impl AppLifecycleState {
    pub const VALUES: [Self; 5] = [
        Self::Detached,
        Self::Resumed,
        Self::Inactive,
        Self::Hidden,
        Self::Paused,
    ];

    pub const fn wire_name(self) -> &'static str {
        match self {
            Self::Detached => "AppLifecycleState.detached",
            Self::Resumed => "AppLifecycleState.resumed",
            Self::Inactive => "AppLifecycleState.inactive",
            Self::Hidden => "AppLifecycleState.hidden",
            Self::Paused => "AppLifecycleState.paused",
        }
    }
}

impl fmt::Display for AppLifecycleState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.wire_name())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AppLifecycleError {
    #[error("unknown application lifecycle state: {0}")]
    UnknownState(String),
    #[error("application lifecycle message is not UTF-8: {0}")]
    InvalidUtf8(#[from] std::str::Utf8Error),
}

impl FromStr for AppLifecycleState {
    type Err = AppLifecycleError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::VALUES
            .into_iter()
            .find(|state| state.wire_name() == value)
            .ok_or_else(|| AppLifecycleError::UnknownState(value.to_owned()))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AppLifecycleStateChange {
    pub previous: Option<AppLifecycleState>,
    pub current: AppLifecycleState,
}

/// Specific lifecycle callback corresponding to one state-change edge.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AppLifecycleCallback {
    Detach,
    Resume,
    Inactive,
    Show,
    Hide,
    Restart,
    Pause,
}

impl AppLifecycleStateChange {
    #[must_use]
    pub const fn callback(self) -> AppLifecycleCallback {
        match (self.previous, self.current) {
            (_, AppLifecycleState::Detached) => AppLifecycleCallback::Detach,
            (_, AppLifecycleState::Resumed) => AppLifecycleCallback::Resume,
            (Some(AppLifecycleState::Hidden), AppLifecycleState::Inactive) => {
                AppLifecycleCallback::Show
            }
            (_, AppLifecycleState::Inactive) => AppLifecycleCallback::Inactive,
            (Some(AppLifecycleState::Paused), AppLifecycleState::Hidden) => {
                AppLifecycleCallback::Restart
            }
            (_, AppLifecycleState::Hidden) => AppLifecycleCallback::Hide,
            (_, AppLifecycleState::Paused) => AppLifecycleCallback::Pause,
        }
    }
}

type LifecycleObserver = Box<dyn FnMut(AppLifecycleStateChange)>;

/// An embedder's instance of the Flutter lifecycle channel. Unlike shutdown,
/// Detached is reversible. The first known state, including a paused
/// initial window, does not synthesize earlier transitions.
#[derive(Default)]
pub struct AppLifecycleBinding {
    state: Option<AppLifecycleState>,
    observers: BTreeMap<u64, LifecycleObserver>,
    next_observer: u64,
}

impl AppLifecycleBinding {
    pub const CHANNEL: &str = "flutter/lifecycle";

    #[must_use]
    pub const fn state(&self) -> Option<AppLifecycleState> {
        self.state
    }

    /// Reads the platform's preexisting startup state at most once.
    pub fn read_initial_state(&mut self, native: &str) -> Result<bool, AppLifecycleError> {
        if self.state.is_some() || native.is_empty() {
            return Ok(false);
        }
        self.transition_to(native.parse()?);
        Ok(true)
    }

    pub fn add_observer(&mut self, callback: impl FnMut(AppLifecycleStateChange) + 'static) -> u64 {
        let id = self.next_observer;
        self.next_observer = id
            .checked_add(1)
            .expect("application lifecycle observer ids exhausted");
        self.observers.insert(id, Box::new(callback));
        id
    }

    pub fn remove_observer(&mut self, id: u64) -> bool {
        self.observers.remove(&id).is_some()
    }

    /// Applies the StringCodec payload of a native lifecycle notification.
    pub fn handle_platform_message(&mut self, payload: &[u8]) -> Result<(), AppLifecycleError> {
        let state = crate::message_codecs::StringCodec::decode_message(Some(payload))?;
        self.transition_to(state.unwrap_or_default().parse()?);
        Ok(())
    }

    /// Supplies the same ordered intermediate edges when the host omits them.
    pub fn transition_to(&mut self, state: AppLifecycleState) {
        let Some(previous) = self.state else {
            self.publish(state);
            return;
        };
        if previous == state {
            return;
        }
        let from = previous as usize;
        let to = state as usize;
        if state == AppLifecycleState::Detached {
            for &step in &AppLifecycleState::VALUES[from + 1..] {
                self.publish(step);
            }
            self.publish(state);
        } else if from > to {
            for &step in AppLifecycleState::VALUES[to..from].iter().rev() {
                self.publish(step);
            }
        } else {
            for &step in &AppLifecycleState::VALUES[from + 1..=to] {
                self.publish(step);
            }
        }
    }

    fn publish(&mut self, current: AppLifecycleState) {
        let previous = self.state.replace(current);
        let event = AppLifecycleStateChange { previous, current };
        for observer in self.observers.values_mut() {
            observer(event);
        }
    }
}
