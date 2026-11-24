//! Flutter 3.50.0 semantics: unspecified enabled is distinct from enabled.
//! The captured neutral fixture is semantics__enabled_default.

use incular_config::Constraints;
use incular_core::Size;
use incular_runtime::Runtime;
use incular_semantics::SemanticState;
use incular_widgets::{Semantics, SizedBox};

fn state_for(enabled: Option<bool>) -> SemanticState {
    let subject = Semantics::new(SizedBox::new().width(120.0).height(40.0))
        .label("Action")
        .button(true)
        .on_tap(|| {});
    let subject = match enabled {
        Some(enabled) => subject.enabled(enabled),
        None => subject,
    };
    let mut runtime = Runtime::new(subject.into()).unwrap();
    runtime
        .run_frame(Constraints::tight(Size::new(120.0, 40.0)))
        .unwrap();
    runtime
        .tree()
        .semantics()
        .iter()
        .find_map(|(_, node)| (node.label.as_deref() == Some("Action")).then(|| node.state.clone()))
        .expect("button semantics node")
}

#[test]
fn default_button_semantics_distinguishes_unset_from_explicitly_enabled() {
    let enabled = state_for(Some(true));
    let disabled = state_for(Some(false));
    let unset = state_for(None);

    assert_ne!(enabled, disabled, "explicit availability control");
    assert_ne!(unset, disabled, "unspecified is not explicitly disabled");
    assert_ne!(unset, enabled, "unspecified is not explicitly enabled");
}
