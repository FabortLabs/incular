use incular_config::{Constraints, TextDirection};
use incular_core::Size;
use incular_runtime::{Application, Runtime};
use incular_widgets::{
    Column, Directionality, Focus, FocusNode, FocusScope, FocusScopeNode, GlobalKey, Text, Widget,
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
