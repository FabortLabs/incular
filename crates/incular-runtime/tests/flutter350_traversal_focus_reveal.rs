use incular_config::Constraints;
use incular_core::{Code, KeyboardEvent, KeyboardKey, NamedKey, Size};
use incular_runtime::Runtime;
use incular_scroll::ScrollController;
use incular_widgets::{
    Center, Column, Directionality, Focus, FocusNode, FocusScope, SingleChildScrollView, SizedBox,
    TextDirection,
};

fn reveals_only_the_target_edge(directional: bool) {
    let scroll = ScrollController::new();
    let nodes = std::array::from_fn::<_, 3, _>(|_| FocusNode::new());
    let content = Column::new(
        nodes
            .iter()
            .map(|node| Focus::new(SizedBox::new().width(120.).height(70.)).node(node.clone())),
    );
    let viewport = SingleChildScrollView::new(FocusScope::new(content)).controller(scroll.clone());
    let root = Directionality::new(
        TextDirection::Ltr,
        Center::new(SizedBox::new().width(120.).height(120.).child(viewport)),
    );
    let mut runtime = Runtime::new(root.into()).unwrap();
    let constraints = Constraints::tight(Size::new(800., 600.));
    runtime.run_frame(constraints).unwrap();
    nodes[0].request_focus();
    runtime.run_frame(constraints).unwrap();
    assert!(nodes[0].has_primary_focus());
    assert_eq!(scroll.offset(), 0.);

    for (reverse, expected, position) in [
        (false, 1, 20.),
        (false, 2, 90.),
        (true, 1, 70.),
        (true, 0, 0.),
    ] {
        let moved = if directional {
            let (key, code) = if reverse {
                (NamedKey::ArrowUp, Code::ArrowUp)
            } else {
                (NamedKey::ArrowDown, Code::ArrowDown)
            };
            let key = KeyboardKey::Named(key);
            let consumed =
                runtime.dispatch_keyboard_event(KeyboardEvent::key_down(key.clone(), code));
            runtime.dispatch_keyboard_event(KeyboardEvent::key_up(key, code));
            consumed
        } else if reverse {
            runtime.previous_focus()
        } else {
            runtime.next_focus()
        };
        assert!(moved);
        for _ in 0..8 {
            runtime.run_frame(constraints).unwrap();
            if !runtime.frame_requested() {
                break;
            }
        }
        assert!(nodes[expected].has_primary_focus(), "target {expected}");
        assert_eq!(scroll.offset(), position, "target {expected}");
    }
}

#[test]
fn sequential_traversal_reveals_next_trailing_or_previous_leading_edge() {
    reveals_only_the_target_edge(false);
}

#[test]
fn directional_traversal_reveals_next_trailing_or_previous_leading_edge() {
    reveals_only_the_target_edge(true);
}
