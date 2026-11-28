//! Mounted policy aggregation and native command boundary for Flutter 3.50.0.
use std::time::Instant;

use incular_config::{
    Constraints,
    ContentSensitivity::{self, AutoSensitive as A, NotSensitive as N, Sensitive as S},
};
use incular_core::{Color, Size};
use incular_platform::{
    ContentSensitivityBackend, MemoryContentSensitivityBackend, WindowOperation,
};
use incular_runtime::{Application, NativeWindowCommand, Signal};
use incular_widgets::{Builder, Column, SensitiveContent, Widget};

type Policy = ContentSensitivity;
type Siblings = Vec<(u64, Policy)>;

fn leaf() -> Widget {
    Widget::box_(Size::new(8., 8.), Color::WHITE)
}

fn scene(changes: Signal<Siblings>) -> Application {
    Application::new(move |_| {
        Builder::new({
            let changes = changes.clone();
            move |_| {
                Column::new(changes.get().into_iter().map(|(key, policy)| {
                    Widget::from(SensitiveContent::with_sensitivity(policy, leaf())).with_key(key)
                }))
                .into()
            }
        })
        .into()
    })
    .unwrap()
}

fn sensitivity_commands(application: &mut Application) -> Vec<Policy> {
    let window = application.primary_window();
    application
        .take_native_window_commands()
        .into_iter()
        .filter_map(|command| match command {
            NativeWindowCommand::Operate(command) if command.window_id == window => {
                match command.operation {
                    WindowOperation::SetContentSensitivity(value) => Some(value),
                    _ => None,
                }
            }
            _ => None,
        })
        .collect()
}

fn pump(application: &mut Application) -> Vec<Policy> {
    let now = Instant::now();
    let window = application.primary_window();
    application
        .run_window_frame_at(window, Constraints::tight(Size::new(200., 120.)), now)
        .unwrap()
        .expect("window frame");
    sensitivity_commands(application)
}

fn baseline(application: &mut Application) -> MemoryContentSensitivityBackend {
    let mut backend = MemoryContentSensitivityBackend::supported();
    backend.apply(A);
    assert!(
        application
            .record_native_content_sensitivity(application.primary_window(), backend.current())
    );
    assert!(sensitivity_commands(application).is_empty());
    backend
}

fn assert_frame(
    application: &mut Application,
    backend: &mut MemoryContentSensitivityBackend,
    line: usize,
    expected: Policy,
    commands: &[Policy],
) {
    let actual = pump(application);
    assert_eq!(actual, commands, "Flutter assertion at source line {line}");
    for command in actual {
        backend.apply(command);
    }
    assert_eq!(backend.current(), expected, "source line {line}");
    assert_eq!(
        application
            .window_diagnostics(application.primary_window())
            .unwrap()
            .content_sensitivity,
        expected,
        "retained window policy at source line {line}",
    );
}

#[test]
fn mounted_siblings_apply_priority_and_removal_only_emits_real_policy_changes() {
    // Each case independently mounts the source scenario and then removes
    // the indicated keyed widget. An omitted removal means the original
    // asserts only the initial result.
    type Case = (
        usize,
        &'static [Policy],
        Option<usize>,
        Policy,
        &'static [Policy],
    );
    let cases: &[Case] = &[
        (53, &[S], None, S, &[]),
        (64, &[S], Some(0), A, &[A]),
        (82, &[S, S], None, S, &[]),
        (104, &[S, S], Some(0), S, &[]),
        (130, &[S, S, S], None, S, &[]),
        (144, &[S, S, S], Some(0), S, &[]),
        (174, &[S, A], None, S, &[]),
        (190, &[S, A], Some(0), A, &[A]),
        (227, &[S, A], Some(1), S, &[]),
        (254, &[S, A, A], Some(1), S, &[]),
        (288, &[S, N], None, S, &[]),
        (304, &[S, N], Some(0), N, &[N]),
        (338, &[S, N], Some(1), S, &[]),
        (364, &[S, N, N], Some(1), S, &[]),
        (397, &[S, A, N], None, S, &[]),
        (419, &[S, A, N], Some(0), A, &[A]),
        (460, &[S, A, N], Some(1), S, &[]),
        (493, &[S, A, N], Some(2), S, &[]),
        (527, &[S, S, A, N], None, S, &[]),
        (551, &[S, S, A, N], Some(0), S, &[]),
        (588, &[S, S, A, N], Some(2), S, &[]),
        (622, &[S, S, A, N], Some(3), S, &[]),
        (657, &[S, A, A, N], Some(1), S, &[]),
        (695, &[S, A, N, N], Some(2), S, &[]),
        (735, &[A, A], None, A, &[]),
        (754, &[A, A], Some(0), A, &[]),
        (786, &[A, N], None, A, &[]),
        (805, &[A, N], Some(0), N, &[N]),
        (834, &[A, N], Some(1), A, &[]),
        (863, &[A, N, N], Some(1), A, &[]),
        (899, &[A, A, N], None, A, &[]),
        (924, &[A, A, N], Some(0), A, &[]),
        (960, &[A, A, N], Some(2), A, &[]),
        (998, &[N, N], None, N, &[]),
        (1017, &[N, N], Some(0), N, &[]),
    ];

    for &(line, initial, remove, end, after) in cases {
        let mut siblings: Siblings = initial
            .iter()
            .enumerate()
            .map(|(index, &policy)| (index as u64, policy))
            .collect();
        let signal = Signal::new(siblings.clone());
        let mut application = scene(signal.clone());
        let mut backend = baseline(&mut application);
        let initial = initial[0]; // These upstream scenarios mount the strongest first.
        let first_commands: &[Policy] = match initial {
            S => &[S],
            A => &[],
            N => &[N],
        };
        assert_frame(
            &mut application,
            &mut backend,
            line,
            initial,
            first_commands,
        );
        if let Some(index) = remove {
            siblings.remove(index);
            signal.set(siblings);
            assert_frame(&mut application, &mut backend, line, end, after);
        }
    }
}

#[test]
fn changing_one_retained_key_never_emits_intermediate_weaker_policy() {
    for (line, old, new, first, after) in [
        (1051, S, S, &[S][..], &[][..]),
        (1082, S, A, &[S], &[A]),
        (1116, S, N, &[S], &[N]),
        (1150, A, S, &[], &[S]),
        (1181, A, A, &[], &[]),
        (1212, A, N, &[], &[N]),
        (1243, N, S, &[N], &[S]),
        (1277, N, A, &[N], &[A]),
        (1311, N, N, &[N], &[]),
    ] {
        let signal = Signal::new(vec![(7, old)]);
        let mut application = scene(signal.clone());
        let mut backend = baseline(&mut application);
        assert_frame(&mut application, &mut backend, line, old, first);
        signal.set(vec![(7, new)]);
        assert_frame(&mut application, &mut backend, line, new, after);
    }
}

#[test]
fn nested_policies_aggregate_without_exposing_an_intermediate_capture_level() {
    for (line, outer, inner, expected, first) in [
        (2060, S, A, S, &[S][..]), // The named upstream fixture actually mounts S around A.
        (2079, A, A, A, &[]),
        (2101, A, N, A, &[]),
        (2123, N, S, S, &[S]),
        (2146, N, A, A, &[]),
        (0, S, S, S, &[S]),
        (0, S, N, S, &[S]),
        (0, N, N, N, &[N]),
    ] {
        let mut application = Application::new(move |_| {
            SensitiveContent::with_sensitivity(
                outer,
                SensitiveContent::with_sensitivity(inner, leaf()),
            )
            .into()
        })
        .unwrap();
        let mut backend = baseline(&mut application);
        assert_frame(&mut application, &mut backend, line, expected, first);
    }
}

#[test]
fn unscoped_application_does_not_overwrite_a_manual_capture_setting() {
    let mut application = scene(Signal::new(Vec::new()));
    assert!(pump(&mut application).is_empty());
    let handle = application
        .window_handle(application.primary_window())
        .unwrap();
    let _request = handle.set_content_sensitivity(S).unwrap();
    assert_eq!(sensitivity_commands(&mut application), [S]);
    assert!(pump(&mut application).is_empty());
    assert_eq!(
        application
            .window_diagnostics(handle.id())
            .unwrap()
            .content_sensitivity,
        S,
    );
}

#[test]
fn keyed_policy_updates_respect_the_unchanged_sibling_without_duplicates() {
    type Case = (
        usize,
        Policy,
        Policy,
        Policy,
        Policy,
        &'static [Policy],
        Policy,
        &'static [Policy],
    );
    // line, changing policy before/after, sibling, initial effective,
    // initial command(s), remaining effective, subsequent command(s).
    let cases: &[Case] = &[
        (1343, S, A, S, S, &[S], S, &[]),
        (1379, S, N, S, S, &[S], S, &[]),
        (1415, S, A, A, S, &[S], A, &[A]),
        (1454, S, N, A, S, &[S], A, &[A]),
        (1493, S, A, N, S, &[S], A, &[A]),
        (1532, S, N, N, S, &[S], N, &[N]),
        (1571, A, S, S, S, &[S], S, &[]),
        (1607, A, N, S, S, &[S], S, &[]),
        (1643, A, S, A, A, &[], S, &[S]),
        (1679, A, N, A, A, &[], A, &[]),
        (1715, A, S, N, A, &[], S, &[S]),
        (1751, A, N, N, A, &[], N, &[N]),
        // The staged registration in Flutter emits N and then S or A on
        // these initial mounts. Incular emits only the resolved protection.
        (1787, N, S, S, S, &[S], S, &[]),
        (1829, N, A, S, S, &[S], S, &[]),
        (1871, N, S, A, A, &[], S, &[S]),
        (1914, N, A, A, A, &[], A, &[]),
        (1956, N, S, N, N, &[N], S, &[S]),
        (1995, N, A, N, N, &[N], A, &[A]),
    ];
    for &(line, old, new, sibling, initial, first, end, after) in cases {
        let signal = Signal::new(vec![(1, old), (2, sibling)]);
        let mut application = scene(signal.clone());
        let mut backend = baseline(&mut application);
        assert_frame(&mut application, &mut backend, line, initial, first);
        signal.set(vec![(1, new), (2, sibling)]);
        assert_frame(&mut application, &mut backend, line, end, after);
    }
}

#[test]
fn moving_a_retained_global_key_and_replacing_a_scope_do_not_leak_registrations() {
    use incular_widgets::{GlobalKey, internal::WidgetTree};
    let key = GlobalKey::new();
    let capture_layer =
        || Widget::from(SensitiveContent::with_sensitivity(S, leaf())).with_key(key.clone());
    let build = |capture_on_left: bool, include: bool| -> Widget {
        let left = Column::new((capture_on_left && include).then(capture_layer));
        let right = Column::new((!capture_on_left && include).then(capture_layer));
        Column::new([
            Widget::from(left).with_key(1_u64),
            Widget::from(right).with_key(2_u64),
        ])
        .into()
    };
    let mut tree = WidgetTree::new();
    let root = tree.mount(build(true, true)).unwrap();
    let limits = Constraints::tight(Size::new(200., 120.));
    tree.layout(limits).unwrap();
    assert_eq!(tree.calculated_content_sensitivity(), Some(S));
    let original = tree.element_with_key(&key.clone().into()).unwrap();
    tree.update(root, build(false, true)).unwrap();
    tree.layout(limits).unwrap();
    assert_eq!(tree.element_with_key(&key.clone().into()), Some(original));
    assert_eq!(tree.calculated_content_sensitivity(), Some(S));
    tree.update(root, build(false, false)).unwrap();
    tree.layout(limits).unwrap();
    assert_eq!(tree.calculated_content_sensitivity(), None);
    let root = tree.mount(Widget::environment_scope(A, leaf())).unwrap();
    tree.layout(limits).unwrap();
    assert_eq!(tree.calculated_content_sensitivity(), Some(A));
    tree.update(root, Widget::environment_scope(123_u32, leaf()))
        .unwrap();
    tree.layout(limits).unwrap();
    assert_eq!(tree.calculated_content_sensitivity(), None);
}
