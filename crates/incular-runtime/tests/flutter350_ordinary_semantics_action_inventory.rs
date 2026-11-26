use std::{cell::RefCell, rc::Rc};

use incular_config::Constraints;
use incular_core::Size;
use incular_runtime::Runtime;
use incular_semantics::SemanticAction;
use incular_widgets::{Semantics, SizedBox};

#[test]
fn ordinary_semantics_node_routes_every_public_flutter_widget_action() {
    let calls = Rc::new(RefCell::new(Vec::new()));
    let observe = calls.clone();
    let descriptor = {
        let callback = |action: SemanticAction| {
            let calls = observe.clone();
            move || calls.borrow_mut().push(action.clone())
        };
        let text = observe.clone();
        let selection = observe.clone();
        let forward = observe.clone();
        let backward = observe.clone();
        Semantics::new(SizedBox::expand())
            .container(true)
            .on_dismiss(callback(SemanticAction::Dismiss))
            .on_tap(callback(SemanticAction::Activate))
            .on_long_press(callback(SemanticAction::LongPress))
            .on_scroll_left(callback(SemanticAction::ScrollLeft))
            .on_scroll_right(callback(SemanticAction::ScrollRight))
            .on_scroll_up(callback(SemanticAction::ScrollUp))
            .on_scroll_down(callback(SemanticAction::ScrollDown))
            .on_increase(callback(SemanticAction::Increment))
            .on_decrease(callback(SemanticAction::Decrement))
            .on_copy(callback(SemanticAction::Copy))
            .on_cut(callback(SemanticAction::Cut))
            .on_paste(callback(SemanticAction::Paste))
            .on_move_cursor_forward_by_character(move |extend_selection| {
                forward
                    .borrow_mut()
                    .push(SemanticAction::MoveCursorForwardByCharacter { extend_selection });
            })
            .on_move_cursor_backward_by_character(move |extend_selection| {
                backward
                    .borrow_mut()
                    .push(SemanticAction::MoveCursorBackwardByCharacter { extend_selection });
            })
            .on_set_selection(move |value| {
                selection.borrow_mut().push(SemanticAction::SetSelection {
                    base: value.base,
                    extent: value.extent,
                });
            })
            .on_set_text(move |value| text.borrow_mut().push(SemanticAction::SetText(value)))
            .on_did_gain_accessibility_focus(callback(SemanticAction::DidGainAccessibilityFocus))
            .on_did_lose_accessibility_focus(callback(SemanticAction::DidLoseAccessibilityFocus))
            .on_focus(callback(SemanticAction::Focus))
            .on_expand(callback(SemanticAction::Expand))
            .on_collapse(callback(SemanticAction::Collapse))
    };
    let mut runtime = Runtime::new(descriptor.into()).unwrap();
    runtime
        .run_frame(Constraints::tight(Size::new(800., 600.)))
        .unwrap();

    let commands = [
        SemanticAction::Dismiss,
        SemanticAction::Activate,
        SemanticAction::LongPress,
        SemanticAction::ScrollLeft,
        SemanticAction::ScrollRight,
        SemanticAction::ScrollUp,
        SemanticAction::ScrollDown,
        SemanticAction::Increment,
        SemanticAction::Decrement,
        SemanticAction::Copy,
        SemanticAction::Cut,
        SemanticAction::Paste,
        SemanticAction::MoveCursorForwardByCharacter {
            extend_selection: true,
        },
        SemanticAction::MoveCursorBackwardByCharacter {
            extend_selection: true,
        },
        SemanticAction::SetSelection { base: 4, extent: 5 },
        SemanticAction::SetText("text".into()),
        SemanticAction::DidGainAccessibilityFocus,
        SemanticAction::DidLoseAccessibilityFocus,
        SemanticAction::Focus,
        SemanticAction::Expand,
        SemanticAction::Collapse,
    ];
    let id = runtime
        .tree()
        .semantics()
        .root()
        .expect("ordinary semantics node");
    let semantic = runtime.tree().semantics().node(id).unwrap();
    assert!(semantic.children.is_empty());
    assert_eq!(semantic.actions.len(), commands.len());
    assert!(
        commands
            .iter()
            .all(|action| semantic.actions.contains(&action.kind()))
    );
    let old_focus = runtime.focused_element();
    for (i, action) in commands.iter().cloned().enumerate() {
        assert!(
            runtime.dispatch_semantic_action(id, action.clone()),
            "{action:?}"
        );
        assert_eq!(&*calls.borrow(), &commands[..=i]);
    }
    assert_eq!(runtime.focused_element(), old_focus);
}
