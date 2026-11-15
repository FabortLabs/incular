//! Flutter's platform undo manager (`services/undo_manager.dart`): the
//! system's undo and redo commands for the focused text field, and the undo
//! availability the framework reports back to the system.

/// Flutter's `UndoDirection`: which way a platform undo command moves
/// through the history.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum UndoDirection {
    /// Reverts the last change.
    Undo,
    /// Reapplies the last reverted change.
    Redo,
}

impl UndoDirection {
    /// Parses the direction names of Flutter's `UndoManagerClient.handleUndo`
    /// message (`"undo"` and `"redo"`).
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "undo" => Some(Self::Undo),
            "redo" => Some(Self::Redo),
            _ => None,
        }
    }
}

/// Commands the runtime sends to the platform undo manager. A host applies
/// them to the system's undo state (iOS's `UIUndoManager`, for example) and
/// ignores them when the system has none.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UndoManagerCommand {
    /// Flutter's `UndoManager.setUndoState`: whether the focused field can
    /// undo and redo. As in Flutter, the runtime sends it only for the iOS
    /// target platform, when a text field gains focus and whenever its
    /// availability changes while it has focus.
    SetUndoState {
        /// Whether an undo is available.
        can_undo: bool,
        /// Whether a redo is available.
        can_redo: bool,
    },
}
