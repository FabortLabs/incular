use incular_config::{Brightness, Constraints, EdgeInsets, NavigationMode, RuntimeEnvironment};
use incular_core::Size;
use incular_runtime::Runtime;
use incular_widgets::{Builder, MediaQuery, SizedBox, ViewController, ViewId, ViewMetrics, Widget};
use std::{cell::RefCell, rc::Rc};

type Reads = Rc<RefCell<Vec<RuntimeEnvironment>>>;

fn observe() -> (Reads, Widget) {
    let reads = Reads::default();
    let recorded = reads.clone();
    let widget = Builder::new(move |context| {
        recorded.borrow_mut().push(MediaQuery::of(context));
        SizedBox::shrink().into()
    })
    .into();
    (reads, widget)
}

fn view() -> ViewController {
    ViewController::new(
        ViewId::PRIMARY,
        ViewMetrics::new(880, 440, 44.),
        RuntimeEnvironment {
            text_scale: 123.,
            brightness: Brightness::Dark,
            accessible_navigation: true,
            ..RuntimeEnvironment::default()
        },
    )
}

fn frame(runtime: &mut Runtime) {
    runtime
        .run_frame(Constraints::tight(Size::new(800., 600.)))
        .unwrap();
}

fn update_view(view: &ViewController, update: impl FnOnce(&mut RuntimeEnvironment)) {
    let mut data = view.data().environment;
    update(&mut data);
    assert!(view.set_environment(data));
}

#[test]
fn from_view_reacts_to_metrics_and_platform_notifications_without_parent_data() {
    let view = view();
    let (reads, child) = observe();
    let outer_absent = Rc::new(RefCell::new(false));
    let absent = outer_absent.clone();
    let view_query = MediaQuery::from_view(view.clone(), child);
    let root = Builder::new(move |context| {
        absent.replace(MediaQuery::maybe_of(context).is_none());
        view_query.clone()
    });
    let mut runtime = Runtime::new(root.into()).unwrap();
    frame(&mut runtime);
    assert!(*outer_absent.borrow());
    assert_eq!(reads.borrow().len(), 1);
    assert_eq!(reads.borrow()[0].text_scale, 123.);

    update_view(&view, |data| data.text_scale = 456.);
    frame(&mut runtime);
    assert_eq!(reads.borrow().len(), 2);
    assert_eq!(reads.borrow()[1].text_scale, 456.);

    assert_eq!(reads.borrow()[1].brightness, Brightness::Dark);
    update_view(&view, |data| data.brightness = Brightness::Light);
    frame(&mut runtime);
    assert_eq!(reads.borrow().len(), 3);
    assert_eq!(reads.borrow()[2].brightness, Brightness::Light);

    assert!(reads.borrow()[2].accessible_navigation);
    update_view(&view, |data| data.accessible_navigation = false);
    frame(&mut runtime);
    assert_eq!(reads.borrow().len(), 4);
    assert!(!reads.borrow()[3].accessible_navigation);

    assert_eq!(reads.borrow()[3].scale_factor, 44.);
    assert!(view.update_metrics(ViewMetrics::new(880, 440, 55.)));
    frame(&mut runtime);
    assert_eq!(reads.borrow().len(), 5);
    assert_eq!(reads.borrow()[4].scale_factor, 55.);
    assert_eq!(reads.borrow()[4].viewport, Size::new(16., 8.));
}

#[test]
fn a_parent_query_shields_platform_notifications_but_not_view_metrics() {
    let view = view();
    let (reads, child) = observe();
    let platform = RuntimeEnvironment {
        text_scale: 44.,
        brightness: Brightness::Dark,
        accessible_navigation: true,
        ..RuntimeEnvironment::default()
    };
    let mut runtime =
        Runtime::new(MediaQuery::new(platform, MediaQuery::from_view(view.clone(), child)).into())
            .unwrap();
    frame(&mut runtime);

    update_view(&view, |data| data.text_scale = 456.);
    frame(&mut runtime);
    update_view(&view, |data| data.brightness = Brightness::Light);
    frame(&mut runtime);
    update_view(&view, |data| data.accessible_navigation = false);
    frame(&mut runtime);
    assert_eq!(reads.borrow().len(), 1);
    assert_eq!(reads.borrow()[0].text_scale, 44.);
    assert_eq!(reads.borrow()[0].brightness, Brightness::Dark);
    assert!(reads.borrow()[0].accessible_navigation);
    assert_eq!(reads.borrow()[0].scale_factor, 44.);

    assert!(view.update_metrics(ViewMetrics::new(880, 440, 55.)));
    frame(&mut runtime);
    assert_eq!(reads.borrow().len(), 2);
    assert_eq!(reads.borrow()[1].scale_factor, 55.);
}

#[test]
fn changing_the_parent_query_updates_the_selected_views_effective_platform_data() {
    let view = view();
    let (reads, child) = observe();
    let scoped = MediaQuery::from_view(view, child);
    let scene = |text_scale| {
        Widget::from(MediaQuery::new(
            RuntimeEnvironment {
                text_scale,
                ..RuntimeEnvironment::default()
            },
            scoped.clone(),
        ))
    };
    let mut runtime = Runtime::new(scene(55.)).unwrap();
    frame(&mut runtime);
    assert_eq!(reads.borrow().len(), 1);
    let root = runtime.tree().root().unwrap();
    runtime.schedule_update(root, scene(66.)).unwrap();
    frame(&mut runtime);
    assert_eq!(reads.borrow().len(), 2);
    assert_eq!(reads.borrow()[0].text_scale, 55.);
    assert_eq!(reads.borrow()[1].text_scale, 66.);
}

#[test]
fn static_projection_keeps_view_insets_and_only_overrides_platform_fields() {
    let view = view();
    update_view(&view, |data| {
        data.view_insets = EdgeInsets::only(0., 0., 0., 4.);
        data.line_height_scale_factor_override = Some(1.5);
    });
    let platform = RuntimeEnvironment {
        viewport: Size::new(999., 999.),
        view_insets: EdgeInsets::all(99.),
        text_scale: 0.,
        navigation_mode: NavigationMode::Directional,
        supports_announce: true,
        ..RuntimeEnvironment::default()
    };
    let scoped = MediaQuery::data_from_view(&view, Some(&platform));
    assert_eq!(scoped.viewport, Size::new(20., 10.));
    assert_eq!(scoped.view_insets.bottom, 4.);
    assert_eq!(scoped.text_scale, 0.);
    assert_eq!(scoped.navigation_mode, NavigationMode::Directional);
    assert!(scoped.supports_announce);
    assert_eq!(scoped.line_height_scale_factor_override, Some(1.5));
    assert_eq!(
        MediaQuery::data_from_view(&view, None).navigation_mode,
        NavigationMode::Traditional,
    );
}

#[test]
fn widgets_app_explicit_media_snapshot_does_not_follow_host_metrics() {
    use incular_widgets::WidgetsApp;
    let (reads, child) = observe();
    let app = WidgetsApp::new(child).environment(RuntimeEnvironment {
        viewport: Size::new(640., 450.),
        view_insets: EdgeInsets::only(0., 0., 0., 80.),
        ..RuntimeEnvironment::default()
    });
    let mut runtime = Runtime::new(app.into()).unwrap();
    frame(&mut runtime);
    assert_eq!(reads.borrow().last().unwrap().view_insets.bottom, 80.);
    assert!(runtime.set_environment(RuntimeEnvironment {
        viewport: Size::new(800., 600.),
        view_insets: EdgeInsets::only(0., 0., 0., 200.),
        ..RuntimeEnvironment::default()
    }));
    frame(&mut runtime);
    assert_eq!(reads.borrow().last().unwrap().view_insets.bottom, 80.);
    assert_eq!(
        reads.borrow().last().unwrap().viewport,
        Size::new(640., 450.)
    );
}
