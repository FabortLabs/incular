use incular_config::{Constraints, TextDirection};
use incular_core::{InputEvent, Offset, PointerPhase, Size};
use incular_runtime::{Application, Runtime, Signal};
use incular_widgets::{
    Column, Directionality, Focus, FocusNode, FocusScope, FocusScopeNode, GestureDetector,
    GlobalKey, SizedBox, Text, Widget,
};

fn status(node: &FocusNode, name: &str, autofocus: bool, key: GlobalKey) -> Widget {
    let label = if node.has_focus() {
        format!("{} FOCUSED", name.to_uppercase())
    } else {
        name.to_owned()
    };
    Widget::from(
        Focus::new(Text::new(label))
            .node(node.clone())
            .autofocus(autofocus),
    )
    .with_key(key)
}

fn app(a: &FocusNode, b: Option<&FocusNode>, autofocus_b: bool) -> (Runtime, [GlobalKey; 2]) {
    let (a, b) = (a.clone(), b.cloned());
    let scope = FocusScopeNode::new();
    let keys = [GlobalKey::new(), GlobalKey::new()];
    let runtime = Application::new(move |_| {
        let a = status(&a, "a", false, keys[0]);
        let children = if let Some(b) = &b {
            Column::new([a, status(b, "b", autofocus_b, keys[1])]).into()
        } else {
            a
        };
        Directionality::new(
            TextDirection::Ltr,
            FocusScope::new(children).node(scope.clone()),
        )
        .into()
    })
    .unwrap()
    .into_runtime();
    (runtime, keys)
}

fn request_from_scope(runtime: &Runtime, key: GlobalKey, node: &FocusNode) {
    let element = runtime.tree().element_with_key(&key.into()).unwrap();
    let scope =
        FocusScope::maybe_of(runtime.tree(), element).expect("nearest retained focus scope");
    assert!(scope.request_focus(node));
}

fn settle(runtime: &mut Runtime) {
    let bounds = Constraints::tight(Size::new(800., 600.));
    for _ in 0..4 {
        runtime.run_frame(bounds).unwrap();
        if !runtime.frame_requested() {
            return;
        }
    }
    assert!(
        !runtime.frame_requested(),
        "focus status kept requesting frames"
    );
}

fn labels(runtime: &Runtime, expected: &[&str]) {
    let mut actual = runtime
        .tree()
        .semantics()
        .iter()
        .filter_map(|(_, node)| node.label.as_deref())
        .collect::<Vec<_>>();
    let mut expected = expected.to_vec();
    actual.sort_unstable();
    expected.sort_unstable();
    assert_eq!(actual, expected);
}

#[test]
fn single_child_focus_updates_the_mounted_status_label() {
    let a = FocusNode::new();
    let (mut runtime, keys) = app(&a, None, false);
    settle(&mut runtime);
    assert!(!a.has_focus());

    request_from_scope(&runtime, keys[0], &a);
    settle(&mut runtime);
    assert!(a.has_focus());
    labels(&runtime, &["A FOCUSED"]);
}

#[test]
fn requesting_the_second_child_unfocuses_the_first_and_rebuilds_both_labels() {
    let a = FocusNode::new();
    let b = FocusNode::new();
    let (mut runtime, keys) = app(&a, Some(&b), false);
    settle(&mut runtime);
    assert!(!a.has_focus() && !b.has_focus());
    labels(&runtime, &["a", "b"]);

    request_from_scope(&runtime, keys[0], &a);
    settle(&mut runtime);
    assert!(a.has_focus() && !b.has_focus());
    labels(&runtime, &["A FOCUSED", "b"]);

    request_from_scope(&runtime, keys[1], &b);
    settle(&mut runtime);
    assert!(!a.has_focus() && b.has_focus());
    labels(&runtime, &["a", "B FOCUSED"]);
}

#[test]
fn second_child_autofocus_updates_its_visible_status_without_claiming_first() {
    let a = FocusNode::new();
    let b = FocusNode::new();
    let (mut runtime, _) = app(&a, Some(&b), true);
    settle(&mut runtime);
    assert!(!a.has_focus() && b.has_focus());
    labels(&runtime, &["a", "B FOCUSED"]);
}

fn interactive_scene(
    a: &FocusNode,
    b: &FocusNode,
    show_a: &Signal<bool>,
    autofocus_a: bool,
) -> (Runtime, [GlobalKey; 2]) {
    let keys = [GlobalKey::new(), GlobalKey::new()];
    let (a, b, show_a) = (a.clone(), b.clone(), show_a.clone());
    let runtime = Application::new(move |_| {
        let row = |node: &FocusNode, name: &str, autofocus, key| {
            let focused = node.clone();
            let child =
                SizedBox::from_size(Size::new(160., 28.)).child(status(node, name, autofocus, key));
            Widget::from(GestureDetector::new(child).on_tap(move || focused.request_focus()))
                .with_key(incular_widgets::internal::Key::from(name))
        };
        let mut rows = Vec::<Widget>::with_capacity(2);
        if show_a.get() {
            rows.push(row(&a, "a", autofocus_a, keys[0]));
        }
        rows.push(row(&b, "b", false, keys[1]));
        Directionality::new(TextDirection::Ltr, Column::new(rows)).into()
    })
    .unwrap()
    .into_runtime();
    (runtime, keys)
}

fn tap(runtime: &mut Runtime, key: GlobalKey) {
    let tree = runtime.tree();
    let element = tree.element_with_key(&key.into()).unwrap();
    let bounds = tree.element_bounds(element).unwrap();
    let position = bounds.origin + Offset::new(bounds.size.width * 0.5, bounds.size.height * 0.5);
    for phase in [PointerPhase::Move, PointerPhase::Down, PointerPhase::Up] {
        let _ = runtime.handle_input(InputEvent::Pointer { phase, position });
    }
    settle(runtime);
}

#[test]
fn tapping_a_focused_child_is_idempotent_and_tapping_siblings_updates_both_labels() {
    let a = FocusNode::new();
    let b = FocusNode::new();
    let (mut runtime, keys) = interactive_scene(&a, &b, &Signal::new(true), true);
    settle(&mut runtime);
    assert!(a.has_primary_focus() && !b.has_focus());
    labels(&runtime, &["A FOCUSED", "b"]);

    tap(&mut runtime, keys[0]);
    assert!(a.has_primary_focus() && !b.has_focus());
    labels(&runtime, &["A FOCUSED", "b"]);

    tap(&mut runtime, keys[1]);
    assert!(b.has_primary_focus() && !a.has_focus());
    labels(&runtime, &["a", "B FOCUSED"]);

    tap(&mut runtime, keys[0]);
    assert!(a.has_primary_focus() && !b.has_focus());
    labels(&runtime, &["A FOCUSED", "b"]);
}

#[test]
fn deleting_a_focused_child_does_not_focus_the_unfocused_sibling() {
    let a = FocusNode::new();
    let b = FocusNode::new();
    let show_a = Signal::new(true);
    let (mut runtime, keys) = interactive_scene(&a, &b, &show_a, false);
    settle(&mut runtime);
    a.request_focus();
    settle(&mut runtime);
    assert!(a.has_primary_focus() && !b.has_focus());
    labels(&runtime, &["A FOCUSED", "b"]);

    let before = runtime.tree().element_with_key(&keys[1].into()).unwrap();
    show_a.set(false);
    settle(&mut runtime);
    assert!(runtime.tree().element_with_key(&keys[0].into()).is_none());
    assert_eq!(
        runtime.tree().element_with_key(&keys[1].into()),
        Some(before)
    );
    assert!(!a.has_focus() && !b.has_focus());
    labels(&runtime, &["b"]);
}

#[test]
fn swapping_two_external_scope_nodes_keeps_primary_focus_and_status_on_the_child() {
    let nodes = [FocusNode::new(), FocusNode::new()];
    let scopes = [FocusScopeNode::new(), FocusScopeNode::new()];
    let root_scope = FocusScopeNode::new();
    let keys = [GlobalKey::new(), GlobalKey::new()];
    let swap = Signal::new(false);
    let mut runtime = Application::new({
        let (nodes, scopes, root_scope, swap) =
            (nodes.clone(), scopes.clone(), root_scope.clone(), swap.clone());
        move |_| {
            let reversed = usize::from(swap.get());
            let branch = |slot: usize, name: &str| {
                let label = status(&nodes[slot], name, false, keys[slot]);
                FocusScope::new(Column::new([label])).node(scopes[slot ^ reversed].clone())
            };
            Directionality::new(TextDirection::Ltr,
                FocusScope::new(Column::new([branch(0, "a"), branch(1, "b")]))
                    .node(root_scope.clone())
            ).into()
        }
    }).unwrap().into_runtime();
    settle(&mut runtime);
    request_from_scope(&runtime, keys[0], &nodes[0]);
    assert!(root_scope.set_first_focus(&scopes[0]));
    settle(&mut runtime);
    assert!(nodes[0].has_primary_focus() && !nodes[1].has_focus());
    labels(&runtime, &["A FOCUSED", "b"]);
    let original = runtime.tree().element_with_key(&keys[0].into()).unwrap();
    assert!(scopes[0].has_focus() && !scopes[1].has_focus());

    swap.set(true);
    settle(&mut runtime);
    assert!(nodes[0].has_primary_focus() && !nodes[1].has_focus());
    assert!(scopes[1].has_focus() && !scopes[0].has_focus());
    let retained = runtime.tree().element_with_key(&keys[0].into()).unwrap();
    assert_eq!(original, retained);
    assert_eq!(FocusScope::maybe_of(runtime.tree(), retained).as_ref(), Some(&scopes[1]));
    labels(&runtime, &["A FOCUSED", "b"]);
}
