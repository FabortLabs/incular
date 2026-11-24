//! PageView clip settings, fractional end padding, and a large-index precision smoke.
use incular_config::{Clip, Constraints};
use incular_core::{Color, Size};
use incular_rendering::PaintCommand;
use incular_runtime::Runtime;
use incular_widgets::{
    Center, Container, Directionality, PageController, PageView, SizedBox, Text, TextDirection,
    Widget,
    internal::{Key, RenderKind},
};

const VIEW: Key = Key::Value(80);
const PAGE: Key = Key::Value(81);
fn dir(w: impl Into<Widget>) -> Widget {
    Directionality::new(TextDirection::Ltr, w).into()
}
fn frame(r: &mut Runtime) -> incular_rendering::DisplayList {
    r.run_frame(Constraints::tight(Size::new(800., 600.)))
        .unwrap()
        .0
}
fn update(r: &mut Runtime, w: Widget) {
    let root = r.tree().root().unwrap();
    r.schedule_update(root, w).unwrap();
}
#[test]
fn page_clip_setting_is_retained_and_disabling_it_removes_the_recorded_clip() {
    let build = |setting: Option<Clip>| {
        let v = PageView::new([Container::new().height(2000.).color(Color::WHITE)]);
        let v = setting.map_or(v.clone(), |clip| v.clip_behavior(clip));
        dir(Widget::from(v).with_key(VIEW))
    };
    let mut r = Runtime::new(build(None)).unwrap();
    let mut actual = Vec::new();
    for (idx, setting) in [None, Some(Clip::AntiAlias), Some(Clip::None)]
        .into_iter()
        .enumerate()
    {
        if idx != 0 {
            update(&mut r, build(setting));
        }
        let list = frame(&mut r);
        let t = r.tree();
        let id = t.render_id(t.element_with_key(&VIEW).unwrap()).unwrap();
        let RenderKind::SliverViewport { config } = t.render_object_kind(id).unwrap() else {
            panic!("expected retained viewport")
        };
        let name = format!("{config:?}");
        let mode = if name.contains("clip_behavior: HardEdge") {
            Clip::HardEdge
        } else if name.contains("clip_behavior: AntiAlias") {
            Clip::AntiAlias
        } else if name.contains("clip_behavior: None") {
            Clip::None
        } else {
            panic!("viewport mode absent from diagnostics: {name}");
        };
        actual.push((
            id,
            mode,
            list.commands()
                .iter()
                .any(|p| matches!(p, PaintCommand::PushClip { .. })),
        ));
    }
    let id = actual[0].0;
    assert_eq!(
        actual,
        [
            (id, Clip::HardEdge, true),
            (id, Clip::AntiAlias, true),
            (id, Clip::None, false)
        ]
    );
}
#[test]
fn page_view_propagates_default_and_disabled_fractional_end_padding() {
    let build = |pad| {
        let v = PageView::new([Widget::from(Text::new("0")).with_key(PAGE)]).viewport_fraction(0.5);
        let v = if pad { v } else { v.pad_ends(false) };
        dir(v)
    };
    let mut r = Runtime::new(build(true)).unwrap();
    let x = |r: &Runtime| {
        let t = r.tree();
        t.element_bounds(t.element_with_key(&PAGE).unwrap())
            .unwrap()
            .origin
            .x
    };
    let _ = frame(&mut r);
    let initial = x(&r);
    update(&mut r, build(false));
    let _ = frame(&mut r);
    assert_eq!([initial, x(&r)], [200., 0.]);
}
#[test]
fn fractional_width_jump_to_last_of_366_pages_does_not_throw() {
    let c = PageController::new().with_initial_page(152);
    let v = PageView::builder(366, |i| {
        Widget::from(SizedBox::new()).with_key(Key::Value(i as u64))
    })
    .controller(c.clone());
    let scene = Center::new(
        SizedBox::new()
            .width(392.72727272727275f64 as f32)
            .child(dir(v)),
    );
    let mut r = Runtime::new(scene.into()).unwrap();
    let _ = frame(&mut r);
    assert!(c.jump_to_page(365));
    let _ = frame(&mut r);
    assert!(r.tree().element_with_key(&Key::Value(365)).is_some());
}
