use incular_config::{Constraints, TextDirection};
use incular_core::Size;
use incular_runtime::{Application, Runtime};
use incular_widgets::{Column, Directionality, Focus, FocusNode, FocusScope, Text, Widget};

fn status(node: &FocusNode, name: &str, autofocus: bool) -> Widget {
    let label = if node.has_focus() {
        format!("{} FOCUSED", name.to_uppercase())
    } else {
        name.to_owned()
    };
    Focus::new(Text::new(label))
        .node(node.clone())
        .autofocus(autofocus)
        .into()
}

fn app(a: &FocusNode, b: Option<&FocusNode>, autofocus_b: bool) -> Runtime {
    let (a, b) = (a.clone(), b.cloned());
    Application::new(move |_| {
        let a = status(&a, "a", false);
        let children = if let Some(b) = &b {
            Column::new([a, status(b, "b", autofocus_b)]).into()
        } else {
            a
        };
        Directionality::new(TextDirection::Ltr, FocusScope::new(children)).into()
    })
    .unwrap()
    .into_runtime()
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
    let mut runtime = app(&a, None, false);
    settle(&mut runtime);
    assert!(!a.has_focus());

    a.request_focus();
    settle(&mut runtime);
    assert!(a.has_focus());
    labels(&runtime, &["A FOCUSED"]);
}

#[test]
fn requesting_the_second_child_unfocuses_the_first_and_rebuilds_both_labels() {
    let a = FocusNode::new();
    let b = FocusNode::new();
    let mut runtime = app(&a, Some(&b), false);
    settle(&mut runtime);
    assert!(!a.has_focus() && !b.has_focus());
    labels(&runtime, &["a", "b"]);

    a.request_focus();
    settle(&mut runtime);
    assert!(a.has_focus() && !b.has_focus());
    labels(&runtime, &["A FOCUSED", "b"]);

    b.request_focus();
    settle(&mut runtime);
    assert!(!a.has_focus() && b.has_focus());
    labels(&runtime, &["a", "B FOCUSED"]);
}

#[test]
fn second_child_autofocus_updates_its_visible_status_without_claiming_first() {
    let a = FocusNode::new();
    let b = FocusNode::new();
    let mut runtime = app(&a, Some(&b), true);
    settle(&mut runtime);
    assert!(!a.has_focus() && b.has_focus());
    labels(&runtime, &["a", "B FOCUSED"]);
}
