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
    assert_eq!(unset.enabled_state(), None);
    assert_eq!(enabled.enabled_state(), Some(true));
    assert_eq!(disabled.enabled_state(), Some(false));
    assert!(unset.enabled && enabled.enabled && !disabled.enabled);
}

#[test]
fn a_retained_node_can_omit_its_flag_again_without_losing_activation() {
    use incular_semantics::SemanticActionKind;
    use incular_widgets::{Center, Widget};
    let build = |state: Option<bool>| {
        let child = Semantics::new(SizedBox::new().width(120.).height(40.))
            .label("Action")
            .button(true)
            .on_tap(|| {});
        Widget::from(Center::new(if let Some(enabled) = state {
            child.enabled(enabled)
        } else {
            child
        }))
    };
    let bounds = Constraints::tight(Size::new(120., 40.));
    let mut runtime = Runtime::new(build(None)).unwrap();
    runtime.run_frame(bounds).unwrap();
    let root = runtime.tree().root().unwrap();
    let mut identity = None;
    for state in [None, Some(true), Some(false), None] {
        runtime.schedule_update(root, build(state)).unwrap();
        runtime.run_frame(bounds).unwrap();
        let (id, node) = runtime
            .tree()
            .semantics()
            .iter()
            .find(|(_, node)| node.label.as_deref() == Some("Action"))
            .unwrap();
        assert_eq!(id, *identity.get_or_insert(id));
        assert_eq!(node.state.enabled_state(), state);
        if state.is_none() {
            assert!(node.actions.contains(&SemanticActionKind::Activate));
        }
    }
}

#[test]
fn a_forced_merge_combines_tri_state_as_flutter_does() {
    use incular_widgets::{Column, MergeSemantics, Widget};
    let leaf = |label, enabled: Option<bool>| {
        let leaf = Semantics::new(SizedBox::square(10.)).label(label);
        Widget::from(if let Some(enabled) = enabled {
            leaf.enabled(enabled)
        } else {
            leaf
        })
    };
    for (first, second, expected) in [
        (None, None, None),
        (None, Some(false), Some(false)),
        (Some(false), Some(true), Some(true)),
        (Some(true), Some(false), Some(true)),
    ] {
        let mut runtime = Runtime::new(
            MergeSemantics::new(Column::new([leaf("First", first), leaf("Second", second)])).into(),
        )
        .unwrap();
        runtime
            .run_frame(Constraints::tight(Size::new(120., 40.)))
            .unwrap();
        let node = runtime
            .tree()
            .semantics()
            .iter()
            .find_map(|(_, node)| {
                node.label
                    .as_deref()
                    .is_some_and(|label| label.contains("First") && label.contains("Second"))
                    .then_some(node)
            })
            .expect("merged node");
        assert_eq!(node.state.enabled_state(), expected);
    }
}
