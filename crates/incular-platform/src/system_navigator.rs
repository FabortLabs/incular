//! Flutter's system navigation commands at the host boundary.

use serde_json::{Value, json};

use crate::flutter_platform_channel::{
    FlutterPlatformChannelError, FlutterPlatformMessenger, invoke_optional_json,
};

/// Options for a route information update. By default the state is null
/// and a host in multi-entry mode creates a new history entry.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RouteInformationUpdate {
    pub state: Value,
    pub replace: bool,
}

/// Opt-in Flutter-compatible host navigation. This requests platform
/// behavior without exiting the process or manipulating browser history
/// directly; the embedder handles supported commands.
pub struct SystemNavigator<M> {
    messenger: M,
}

impl<M: FlutterPlatformMessenger> SystemNavigator<M> {
    pub const PLATFORM_CHANNEL: &str = "flutter/platform";
    pub const NAVIGATION_CHANNEL: &str = "flutter/navigation";

    #[must_use]
    pub const fn new(messenger: M) -> Self {
        Self { messenger }
    }

    /// Removes the current app instance if the enclosing platform supports
    /// it. Hosts that support the option may animate the transition.
    pub async fn pop(
        &mut self,
        animated: Option<bool>,
    ) -> Result<(), FlutterPlatformChannelError<M::Error>> {
        invoke_optional_json(
            &mut self.messenger,
            Self::PLATFORM_CHANNEL,
            "SystemNavigator.pop",
            animated.map_or(Value::Null, Value::Bool),
        )
        .await
    }

    pub async fn select_single_entry_history(
        &mut self,
    ) -> Result<(), FlutterPlatformChannelError<M::Error>> {
        invoke_optional_json(
            &mut self.messenger,
            Self::NAVIGATION_CHANNEL,
            "selectSingleEntryHistory",
            Value::Null,
        )
        .await
    }

    pub async fn select_multi_entry_history(
        &mut self,
    ) -> Result<(), FlutterPlatformChannelError<M::Error>> {
        invoke_optional_json(
            &mut self.messenger,
            Self::NAVIGATION_CHANNEL,
            "selectMultiEntryHistory",
            Value::Null,
        )
        .await
    }

    /// Notifies the host of an already serialized route URI and history state.
    pub async fn route_information_updated(
        &mut self,
        uri: &str,
        update: RouteInformationUpdate,
    ) -> Result<(), FlutterPlatformChannelError<M::Error>> {
        invoke_optional_json(
            &mut self.messenger,
            Self::NAVIGATION_CHANNEL,
            "routeInformationUpdated",
            json!({ "uri": uri, "state": update.state, "replace": update.replace }),
        )
        .await
    }
}
