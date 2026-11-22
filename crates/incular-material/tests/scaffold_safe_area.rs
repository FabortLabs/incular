//! Scaffold and AppBar apply the safe insets the way Flutter does: the app
//! bar pads its toolbar, the body drops the insets its neighbours pad, bars
//! add the bottom inset, and the FAB floats above the bottom region.

use incular_config::{Constraints, EdgeInsets, RuntimeEnvironment};
use incular_core::{Color, Rect, Size};
use incular_material::{AppBar, NavigationBar, NavigationDestination, Scaffold};
use incular_runtime::Runtime;
use incular_widgets::{Builder, Directionality, Semantics, TextDirection, Widget};

/// A 300x600 view with a 24 pixel status bar and a 16 pixel bottom inset.
const SAFE: EdgeInsets = EdgeInsets {
    left: 0.,
    top: 24.,
    right: 0.,
    bottom: 16.,
};

fn view() -> Size {
    Size::new(300., 600.)
}

fn labeled(size: Size, label: &str) -> Widget {
    Semantics::new(Widget::box_(size, Color::WHITE))
        .role(incular_semantics::Role::Group)
        .label(label)
        .into()
}

/// A body that reports the safe insets it sees as its label, `Body top bottom`.
fn reporting_body() -> Widget {
    Builder::new(|context| {
        let insets = context.safe_insets();
        Semantics::new(Widget::box_(view(), Color::WHITE))
            .role(incular_semantics::Role::Group)
            .label(format!("Body {} {}", insets.top, insets.bottom))
            .into()
    })
    .into()
}

fn mount(scaffold: Scaffold) -> Runtime {
    let mut runtime =
        Runtime::new(Directionality::new(TextDirection::Ltr, scaffold).into()).expect("mount");
    runtime.set_environment(RuntimeEnvironment {
        viewport: view(),
        view_padding: SAFE,
        safe_insets: SAFE,
        ..RuntimeEnvironment::default()
    });
    runtime
        .run_frame(Constraints::tight(view()))
        .expect("frame");
    runtime
}

fn bounds(runtime: &Runtime, label: &str) -> Rect {
    runtime
        .tree()
        .semantics()
        .iter()
        .find_map(|(_, node)| (node.label.as_deref() == Some(label)).then_some(node.bounds))
        .unwrap_or_else(|| panic!("labeled slot `{label}`"))
}

fn app_bar(primary: bool) -> AppBar {
    AppBar::new(labeled(Size::new(100., 40.), "Title"))
        .toolbar_height(40.)
        .primary(primary)
}

#[test]
fn app_bar_pads_its_toolbar_and_the_body_starts_below_it() {
    let runtime = mount(Scaffold::new(reporting_body()).app_bar(app_bar(true)));

    assert_eq!(bounds(&runtime, "Title").origin.y, 24.);
    // The slot is the 40 pixel toolbar plus the 24 pixel status bar.
    let body = bounds(&runtime, "Body 0 16");
    assert_eq!(body.origin.y, 64.);
    assert_eq!(body.size.height, 600. - 64.);
}

#[test]
fn body_drops_the_top_inset_only_under_an_app_bar() {
    let with_bar = mount(Scaffold::new(reporting_body()).app_bar(app_bar(true)));
    assert_eq!(bounds(&with_bar, "Body 0 16").origin.y, 64.);

    let without_bar = mount(Scaffold::new(reporting_body()));
    let body = bounds(&without_bar, "Body 24 16");
    assert_eq!(body.origin.y, 0.);
    assert_eq!(body.size.height, 600.);
}

#[test]
fn navigation_bar_adds_the_bottom_inset_and_the_body_drops_it() {
    let bar = NavigationBar::new([NavigationDestination::new(
        Widget::box_(Size::new(10., 10.), Color::WHITE),
        "Home",
    )]);
    let fab = labeled(Size::new(20., 20.), "Fab");
    let runtime = mount(
        Scaffold::new(reporting_body())
            .bottom_navigation_bar(bar)
            .floating_action_button(fab),
    );

    // The body stops at the bar's top: 80 pixel bar plus the 16 pixel inset.
    let body = bounds(&runtime, "Body 24 0");
    assert_eq!(body.size.height, 600. - 96.);
    // The FAB floats 16 pixels above the bar.
    let fab = bounds(&runtime, "Fab");
    assert_eq!(fab.origin.y + fab.size.height, 600. - 96. - 16.);
}

#[test]
fn fab_floats_above_the_bottom_inset_without_a_bar() {
    let runtime = mount(
        Scaffold::new(reporting_body()).floating_action_button(labeled(Size::new(20., 20.), "Fab")),
    );

    let fab = bounds(&runtime, "Fab");
    // 16 pixel margin plus the 16 pixel bottom inset.
    assert_eq!(fab.origin.y + fab.size.height, 600. - 16. - 16.);
    assert_eq!(fab.origin.x + fab.size.width, 300. - 16.);
}

#[test]
fn primary_false_disables_the_top_padding() {
    let bar_only = mount(Scaffold::new(reporting_body()).app_bar(app_bar(false)));
    assert_eq!(bounds(&bar_only, "Title").origin.y, 0.);
    assert_eq!(bounds(&bar_only, "Body 0 16").origin.y, 40.);

    let scaffold_only = mount(
        Scaffold::new(reporting_body())
            .primary(false)
            .app_bar(app_bar(true)),
    );
    assert_eq!(bounds(&scaffold_only, "Title").origin.y, 0.);
    assert_eq!(bounds(&scaffold_only, "Body 0 16").origin.y, 40.);
}
