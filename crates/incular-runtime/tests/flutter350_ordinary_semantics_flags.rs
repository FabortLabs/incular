use incular_config::Constraints;
use incular_core::{Offset, Rect, Size};
use incular_runtime::Runtime;
use incular_semantics::{CheckedState, SemanticAnnotationFlag as Flag};
use incular_widgets::{Semantics, SizedBox, Widget};

const BOUNDS: Size = Size {
    width: 800.,
    height: 600.,
};

fn scene(variant: u8) -> Widget {
    let base = Semantics::new(SizedBox::expand());
    match variant {
        0 | 3 => base
            .container(true)
            .explicit_child_nodes(true)
            .enabled(true)
            .hidden(true)
            .checked(if variant == 0 {
                CheckedState::Checked
            } else {
                CheckedState::Indeterminate
            })
            .selected(true)
            .button(true)
            .slider(true)
            .keyboard_key(true)
            .link(true)
            .text_field(true)
            .read_only(true)
            .focused(true)
            .focusable(true)
            .in_mutually_exclusive_group(true)
            .header(true)
            .obscured(true)
            .multiline(true)
            .scopes_route(true)
            .names_route(true)
            .image(true)
            .live_region(true)
            .expanded(true)
            .required(true),
        1 => base.container(true).scopes_route(false),
        _ => base.toggled(true),
    }
    .into()
}

#[test]
fn regular_semantics_matches_all_four_pinned_flag_snapshots() {
    let mut runtime = Runtime::new(scene(0)).unwrap();
    runtime.run_frame(Constraints::tight(BOUNDS)).unwrap();
    let root = runtime.tree().root().unwrap();
    let flags = [
        Flag::Button,
        Flag::Slider,
        Flag::KeyboardKey,
        Flag::Link,
        Flag::TextField,
        Flag::MutuallyExclusiveGroup,
        Flag::Header,
        Flag::ScopesRoute,
        Flag::NamesRoute,
        Flag::Image,
    ];
    for variant in 0..4 {
        runtime.schedule_update(root, scene(variant)).unwrap();
        runtime.run_frame(Constraints::tight(BOUNDS)).unwrap();
        let tree = runtime.tree().semantics();
        let semantic = tree.node(tree.root().unwrap()).unwrap();
        assert_eq!(tree.iter().count(), 1);
        assert!(semantic.children.is_empty() && semantic.actions.is_empty());
        assert_eq!(
            semantic.bounds,
            Rect::from_origin_size(Offset::ZERO, BOUNDS)
        );
        let state = &semantic.state;
        if variant == 0 || variant == 3 {
            let checked = if variant == 0 {
                CheckedState::Checked
            } else {
                CheckedState::Indeterminate
            };
            assert_eq!(state.checked, Some(checked));
            assert_eq!(state.toggled, None);
            assert!(state.enabled && state.has_enabled_state);
            assert!(state.hidden && state.selected && state.focusable && state.focused);
            assert!(state.read_only && state.obscured && state.multiline);
            assert!(state.live_region && state.required);
            assert_eq!(state.expanded, Some(true));
            for flag in flags {
                assert_eq!(state.annotation_flags.get(flag), Some(true), "{flag:?}");
            }
        } else {
            assert_eq!(state.checked, None);
            assert_eq!(state.toggled, if variant == 2 { Some(true) } else { None });
            assert_eq!(state.expanded, None);
            assert!(
                !state.required
                    && !state.hidden
                    && !state.selected
                    && !state.focusable
                    && !state.focused
            );
            assert!(
                !state.has_enabled_state
                    && !state.read_only
                    && !state.obscured
                    && !state.multiline
                    && !state.live_region
            );
            for flag in flags {
                assert_ne!(state.annotation_flags.get(flag), Some(true), "{flag:?}");
            }
        }
    }
    assert!(runtime.focused_element().is_none());
}
