use std::{cell::RefCell, rc::Rc};

use incular_config::Constraints;
use incular_core::{Color, PointerDeviceKind, Size};
use incular_runtime::{Application, Runtime, Signal};
use incular_widgets::{
    Builder, Center, GlobalKey, ScrollBehavior, ScrollConfiguration, SizedBox, Text, Widget,
    WidgetsApp,
};

fn draw(runtime: &mut Runtime) {
    runtime
        .run_frame(Constraints::tight(Size::new(800., 600.)))
        .unwrap();
}

fn builder_only(view: impl Into<Widget>) -> WidgetsApp {
    let view = view.into();
    WidgetsApp::empty().builder(move |_| view.clone())
}

#[test]
fn a_builder_only_app_mounts_its_keyed_root() {
    let app = GlobalKey::new();
    let leaf = GlobalKey::new();
    let body = Widget::from(SizedBox::square(20.)).with_key(leaf);
    let root = Widget::from(builder_only(body)).with_key(app);
    let mut runtime = Runtime::new(root).unwrap();
    draw(&mut runtime);
    let tree = runtime.tree();
    assert!(tree.element_with_key(&app.into()).is_some());
    assert!(tree.element_with_key(&leaf.into()).is_some());
}

#[test]
fn the_builder_inherits_default_scroll_behavior_without_a_configuration() {
    let observed = Rc::new(RefCell::new(None));
    let record = observed.clone();
    let body = Builder::new(move |context| {
        *record.borrow_mut() = Some(ScrollConfiguration::of(context));
        SizedBox::shrink().into()
    });
    let mut runtime = Runtime::new(builder_only(body).into()).unwrap();
    draw(&mut runtime);
    assert_eq!(*observed.borrow(), Some(ScrollBehavior::new()));
}

#[test]
fn a_builder_only_widgets_app_lays_out_at_zero_area() {
    let key = GlobalKey::new();
    let app = builder_only(Text::new("X")).color(Color::from_argb32(0xFFAABBCC));
    let app = Widget::from(app).with_key(key);
    let root = Center::new(SizedBox::shrink().child(app));
    let mut runtime = Runtime::new(root.into()).unwrap();
    draw(&mut runtime);
    let id = runtime.tree().element_with_key(&key.into()).unwrap();
    assert_eq!(runtime.tree().element_bounds(id).unwrap().size, Size::ZERO);
}

#[test]
fn a_mounted_builder_observes_updates_to_its_nearest_scroll_behavior() {
    let observed = Rc::new(RefCell::new(Vec::new()));
    let record = observed.clone();
    let body = Builder::new(move |context| {
        record.borrow_mut().push(ScrollConfiguration::of(context));
        SizedBox::shrink().into()
    });
    let app: Widget = builder_only(body).into();
    let mouse = Signal::new(false);
    let state = mouse.clone();
    let mut runtime = Application::new(move |_| {
        let device = if state.get() {
            PointerDeviceKind::Mouse
        } else {
            PointerDeviceKind::Touch
        };
        ScrollConfiguration::new(app.clone())
            .behavior(ScrollBehavior::new().drag_devices([device]))
            .into()
    })
    .unwrap()
    .into_runtime();
    draw(&mut runtime);
    assert_eq!(
        observed.borrow().last().unwrap().get_drag_devices(),
        &[PointerDeviceKind::Touch],
    );
    mouse.set(true);
    draw(&mut runtime);
    assert_eq!(
        observed.borrow().last().unwrap().get_drag_devices(),
        &[PointerDeviceKind::Mouse],
    );
}
