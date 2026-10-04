//! Android semantic adapter for a native Incular host.
//!
//! [`AndroidAccessibilityAdapter`] projects retained semantics into incremental
//! updates for the host's JNI/View bridge and validates actions returned by it.
//! Application lifecycle, input, surfaces and the native runner are not provided.

use incular_accessibility::{
    MobileAccessibilityProjection, MobileAccessibilityUpdate, SemanticAction,
    SemanticActionRequest, SemanticsTree,
};

/// Android-facing accessibility adapter. The host's JNI/View layer consumes
/// the stable-node update and emits actions back through [`Self::action`].
/// Keeping this adapter data-only makes it usable by both a View host and a
/// Compose-style bridge without leaking either API into the core runtime.
#[derive(Default)]
pub struct AndroidAccessibilityAdapter {
    projection: MobileAccessibilityProjection,
}

impl AndroidAccessibilityAdapter {
    /// Creates an inactive adapter; call [`Self::activate`] before syncing.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Enables projection and requests a full snapshot on the next sync.
    pub fn activate(&mut self) {
        self.projection.activate();
    }

    /// Pauses projection and action translation until the next activation.
    pub fn deactivate(&mut self) {
        self.projection.deactivate();
    }

    /// Returns a full snapshot after activation, then incremental changes.
    /// Returns `None` while inactive or when the tree has not changed.
    pub fn sync(&mut self, tree: &SemanticsTree) -> Option<MobileAccessibilityUpdate> {
        self.projection.sync(tree)
    }

    /// Translates an action supported by a node in the last projected snapshot.
    /// Returns `None` for inactive, stale or unsupported native requests.
    pub fn action(
        &mut self,
        native_id: u64,
        action: SemanticAction,
    ) -> Option<SemanticActionRequest> {
        self.projection.translate_action(native_id, action)
    }

    /// Returns projection and action-translation counters.
    #[must_use]
    pub fn diagnostics(&self) -> incular_accessibility::AccessibilityDiagnostics {
        self.projection.diagnostics()
    }

    /// Returns the host's node identity once the semantic node has been projected.
    #[must_use]
    pub fn native_node_id(&self, node: incular_accessibility::SemanticNodeId) -> Option<u64> {
        self.projection.native_node_id(node)
    }
}
