//! Read-only DevTools inspection requests answered from a window's retained
//! tree: runtime counters, semantics, compositor layers and work counts.
//!
//! These additions live apart from the frame pipeline; nothing here marks the
//! tree dirty or triggers layout, paint or composition.

use crate::application::Application;
use incular_devtools_protocol::{
    DevWindowId, ErrorCode, LayerNodeWire, RuntimeDiagnosticsWire, SemanticsNodeWire,
    WorkCountEntry,
};
use incular_platform::WindowId;
use incular_widgets::internal::WidgetTree;

impl Application {
    /// Reads the retained tree of one window, or reports a stale ID.
    fn with_window_tree<T>(
        &self,
        window: DevWindowId,
        read: impl FnOnce(&WidgetTree) -> T,
    ) -> Result<T, ErrorCode> {
        let id = WindowId::from_parts(
            window.index().saturating_sub(1) as u32,
            window.generation() as u32,
        );
        let registry = self.registry.borrow();
        let record = registry.get(id).ok_or(ErrorCode::StaleId)?;
        Ok(read(record.runtime.tree()))
    }

    /// Cumulative widget, text, semantics and compositor counters for one window.
    pub fn devtools_runtime_diagnostics(
        &self,
        window: DevWindowId,
    ) -> Result<RuntimeDiagnosticsWire, ErrorCode> {
        self.with_window_tree(window, |tree| tree.devtools_runtime_counters())
    }

    /// Bounded semantics tree snapshot; the flag reports truncation.
    pub fn devtools_semantics_tree(
        &self,
        window: DevWindowId,
    ) -> Result<(Vec<SemanticsNodeWire>, bool), ErrorCode> {
        self.with_window_tree(window, |tree| tree.devtools_semantics_nodes())
    }

    /// Bounded compositor layer snapshot; the flag reports truncation.
    pub fn devtools_layer_tree(
        &self,
        window: DevWindowId,
    ) -> Result<(Vec<LayerNodeWire>, bool), ErrorCode> {
        self.with_window_tree(window, |tree| tree.devtools_layer_nodes())
    }

    /// Per-element work counters ranked by build count, at most `limit`
    /// (itself capped at 200).
    pub fn devtools_work_counts(
        &self,
        window: DevWindowId,
        limit: usize,
    ) -> Result<Vec<WorkCountEntry>, ErrorCode> {
        self.with_window_tree(window, |tree| tree.devtools_work_counts(limit))
    }
}
