//! Material input decoration and field presentation primitives.

mod input;
mod magnifier;
mod text_fields;
mod text_selection_handles;
mod text_selection_toolbar;
mod vocabulary;

pub use incular_widgets::DesktopTextSelectionToolbarLayoutDelegate;
pub use input::*;
pub use magnifier::{Magnifier, TextMagnifier, TextMagnifierPosition};
pub use text_fields::{Autocomplete, SelectableText, SelectionArea, TextField, TextFormField};
pub use text_selection_handles::default_text_selection_handles_builder;
pub use text_selection_toolbar::{
    AdaptiveTextSelectionToolbar, CupertinoDesktopTextSelectionToolbar,
    CupertinoDesktopTextSelectionToolbarButton, DesktopTextSelectionToolbar,
    DesktopTextSelectionToolbarButton, TextSelectionToolbar, TextSelectionToolbarTextButton,
    default_text_context_menu_builder,
};
pub use vocabulary::*;
