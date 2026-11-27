use std::{
    cell::Cell,
    rc::Rc,
    time::{Duration, Instant},
};

use incular_config::{Alignment, AlignmentDirectional, Constraints, TextDirection};
use incular_core::{Offset, Size, Transform};
use incular_widgets::internal::{ElementId, Key, WidgetTree};
use incular_widgets::{AnimatedContainer, Center, Directionality, SizedBox, Widget};

const DURATION: Duration = Duration::from_millis(200);

struct Scene {
    tree: WidgetTree,
    root: ElementId,
    base: Instant,
}

impl Scene {
    fn new(widget: Widget) -> Self {
        let mut tree = WidgetTree::new();
        let root = tree.mount(widget).unwrap();
        let mut out = Self {
            tree,
            root,
            base: Instant::now(),
        };
        out.at(0);
        out.at(0);
        out
    }

    fn at(&mut self, millis: u64) {
        self.tree
            .update_compositor(self.base + Duration::from_millis(millis))
            .unwrap();
        self.tree
            .layout(Constraints::tight(Size::new(800., 600.)))
            .unwrap();
    }

    fn replace(&mut self, widget: Widget, millis: u64) {
        self.tree.update(self.root, widget).unwrap();
        self.at(millis);
        self.at(millis);
    }

    fn target(&self, key: &Key) -> ElementId {
        self.tree.element_with_key(key).unwrap()
    }

    fn local_size(&self, key: &Key) -> Size {
        self.tree
            .render_size(self.tree.render_id(self.target(key)).unwrap())
            .unwrap()
    }

    fn point(&self, key: &Key, point: Offset) -> Offset {
        self.tree
            .element_local_to_global(self.target(key), point, None)
            .unwrap()
    }
}

#[test]
fn direction_change_resolves_current_alignment_without_restarting() {
    direction_change(false);
}

#[test]
fn direction_change_resolves_current_transform_pivot_without_restarting() {
    direction_change(true);
}

fn direction_change(pivot: bool) {
    let key = Key::from("animated container child");
    let completions = Rc::new(Cell::new(0));
    let build = |direction, value: AlignmentDirectional| {
        let child = Widget::from(SizedBox::new().width(100.).height(200.)).with_key(key.clone());
        let ended = completions.clone();
        let animated = AnimatedContainer::new(DURATION)
            .child(child)
            .on_end(move || ended.set(ended.get() + 1));
        let content: Widget = if pivot {
            Center::new(
                animated
                    .transform(Transform::scale(0.5))
                    .transform_alignment(value),
            )
            .into()
        } else {
            animated.alignment(value).into()
        };
        Directionality::new(direction, content).into()
    };
    let expected = if pivot {
        [350., 362.5, 387.5, 375., 350.]
    } else {
        [0., 175., 525., 350., 0.]
    };
    let mut scene = Scene::new(build(
        TextDirection::Ltr,
        AlignmentDirectional::CENTER_START,
    ));
    let target = scene.target(&key);
    let verify = |scene: &Scene, left| {
        assert_eq!(scene.target(&key), target);
        assert_eq!(scene.local_size(&key), Size::new(100., 200.));
        assert_eq!(scene.point(&key, Offset::ZERO).x, left);
    };
    verify(&scene, expected[0]);
    scene.replace(
        build(TextDirection::Ltr, AlignmentDirectional::CENTER_END),
        1_000,
    );
    scene.at(1_050);
    verify(&scene, expected[1]);
    scene.replace(
        build(TextDirection::Rtl, AlignmentDirectional::CENTER_END),
        1_050,
    );
    verify(&scene, expected[2]);
    assert_eq!(completions.get(), 0);
    scene.at(1_100);
    verify(&scene, expected[3]);
    scene.at(1_200);
    verify(&scene, expected[4]);
    scene.at(1_201);
    assert_eq!(completions.get(), 1);
    scene.at(1_400);
    assert_eq!(completions.get(), 1);
}

#[test]
fn physical_to_directional_rtl_alignment_matches_every_sample() {
    use incular_widgets::AlignmentGeometry;
    let key = Key::from("rtl target");
    let build = |geometry: AlignmentGeometry| {
        Directionality::new(
            TextDirection::Rtl,
            AnimatedContainer::new(DURATION).alignment(geometry).child(
                Widget::from(SizedBox::new().width(100.).height(200.)).with_key(key.clone()),
            ),
        )
        .into()
    };
    let mut scene = Scene::new(build(Alignment::TOP_RIGHT.into()));
    let verify = |scene: &Scene, top| {
        assert_eq!(scene.local_size(&key), Size::new(100., 200.));
        assert_eq!(
            scene.point(&key, Offset::new(100., 0.)),
            Offset::new(800., top)
        );
    };
    verify(&scene, 0.);
    scene.replace(build(AlignmentDirectional::BOTTOM_START.into()), 1_000);
    verify(&scene, 0.);
    scene.at(1_100);
    verify(&scene, 200.);
    scene.at(1_600);
    verify(&scene, 400.);
}

#[test]
fn visual_to_directional_rtl_padding_matches_every_sample() {
    use incular_config::EdgeInsetsDirectional;
    use incular_widgets::EdgeInsetsGeometry;
    let key = Key::from("expanded rtl target");
    let build = |padding: EdgeInsetsGeometry| {
        Directionality::new(
            TextDirection::Rtl,
            AnimatedContainer::new(DURATION)
                .padding(padding)
                .child(Widget::from(SizedBox::expand()).with_key(key.clone())),
        )
        .into()
    };
    let mut scene = Scene::new(build(EdgeInsetsGeometry::only(0., 0., 50., 0.)));
    let verify = |scene: &Scene, width| {
        assert_eq!(scene.local_size(&key), Size::new(width, 600.));
        assert_eq!(
            scene.point(&key, Offset::new(width, 0.)),
            Offset::new(width, 0.)
        );
    };
    verify(&scene, 750.);
    scene.replace(
        build(EdgeInsetsDirectional::only(100., 0., 0., 0.).into()),
        1_000,
    );
    verify(&scene, 750.);
    scene.at(1_100);
    verify(&scene, 725.);
    scene.at(1_600);
    verify(&scene, 700.);
}

#[test]
fn physical_transform_pivot_moves_without_changing_layout_size() {
    let key = Key::from("transform target");
    let build = |pivot| {
        Center::new(
            AnimatedContainer::new(DURATION)
                .transform(Transform::scale(0.5))
                .transform_alignment(pivot)
                .child(
                    Widget::from(SizedBox::new().width(100.).height(200.)).with_key(key.clone()),
                ),
        )
        .into()
    };
    let mut scene = Scene::new(build(Alignment::TOP_LEFT));
    let verify = |scene: &Scene, x, y| {
        assert_eq!(scene.local_size(&key), Size::new(100., 200.));
        assert_eq!(scene.point(&key, Offset::ZERO), Offset::new(x, y));
    };
    verify(&scene, 350., 200.);
    scene.replace(build(Alignment::BOTTOM_RIGHT), 1_000);
    verify(&scene, 350., 200.);
    scene.at(1_100);
    verify(&scene, 375., 250.);
    scene.at(1_600);
    verify(&scene, 400., 300.);
}

#[test]
fn a_second_run_only_animates_the_changed_dimension() {
    use incular_widgets::Text;
    let key = Key::from("animated text");
    let build = |width, height| {
        Directionality::new(
            TextDirection::Ltr,
            Center::new(
                AnimatedContainer::new(DURATION)
                    .width(width)
                    .height(height)
                    .child(Widget::from(Text::new("X")).with_key(key.clone())),
            ),
        )
        .into()
    };
    let mut scene = Scene::new(build(100., 100.));
    scene.at(100);
    assert_eq!(scene.local_size(&key), Size::new(100., 100.));
    scene.replace(build(200., 200.), 1_100);
    scene.at(1_200);
    assert_eq!(scene.local_size(&key), Size::new(150., 150.));
    scene.at(2_200);
    assert_eq!(scene.local_size(&key), Size::new(200., 200.));
    scene.replace(build(200., 100.), 2_200);
    scene.at(2_300);
    assert_eq!(scene.local_size(&key), Size::new(200., 150.));
    scene.at(3_300);
    assert_eq!(scene.local_size(&key), Size::new(200., 100.));
}

#[test]
fn clipping_defaults_to_none_and_explicit_antialias_changes_paint_immediately() {
    use incular_config::Clip;
    use incular_core::Color;
    use incular_rendering::PaintCommand;
    use incular_widgets::BoxDecoration;
    let build = |clip: Option<Clip>| {
        let container = AnimatedContainer::new(DURATION)
            .decoration(BoxDecoration::new().color(Color::rgba(0xed, 0x1d, 0x7f, 0xff)));
        clip.map_or(container.clone(), |mode| container.clip_behavior(mode))
            .into()
    };
    let mut scene = Scene::new(build(None::<Clip>));
    let clip_count = |scene: &mut Scene| {
        scene
            .tree
            .paint()
            .commands()
            .iter()
            .filter(|command| {
                matches!(
                    command,
                    PaintCommand::PushClip { .. }
                        | PaintCommand::PushClipRRect { .. }
                        | PaintCommand::PushClipPath { .. }
                )
            })
            .count()
    };
    assert_eq!(clip_count(&mut scene), 0);
    scene.replace(build(Some(Clip::AntiAlias)), 1_000);
    assert_eq!(clip_count(&mut scene), 1);
    scene.at(1_200);
    assert_eq!(clip_count(&mut scene), 1);
}

#[test]
fn zero_area_does_not_crash_on_frame_or_settle() {
    use incular_widgets::Text;
    let key = Key::from("zero container");
    let container =
        Widget::from(AnimatedContainer::new(DURATION).child(Text::new("X"))).with_key(key.clone());
    let widget = Directionality::new(
        TextDirection::Ltr,
        Center::new(SizedBox::shrink().child(container)),
    );
    let mut scene = Scene::new(widget.into());
    scene.at(100);
    scene.at(300);
    assert_eq!(scene.local_size(&key), Size::ZERO);
}

#[test]
fn full_container_debug_description_is_one_line() {
    use incular_config::EdgeInsets;
    use incular_core::Color;
    use incular_widgets::BoxDecoration;
    let container = AnimatedContainer::new(DURATION)
        .constraints(Constraints::tight(Size::new(17., 23.)))
        .decoration(BoxDecoration::new().color(Color::rgba(0, 255, 0, 255)))
        .foreground_decoration(BoxDecoration::new().color(Color::rgba(0, 0, 255, 127)))
        .margin(EdgeInsets::all(10.))
        .padding(EdgeInsets::all(7.))
        .transform(Transform::translation(Offset::new(4., 3.)))
        .width(50.)
        .height(75.)
        .curve(incular_animation::Curves::EASE);
    let text = format!("{container:?}");
    assert_eq!(text.trim(), text);
    assert!(!text.contains('\n'));
    assert!(text.starts_with("AnimatedContainer"));
    for field in [
        "constraints",
        "decoration",
        "foreground_decoration",
        "margin",
        "padding",
        "transform",
        "duration",
        "curve",
    ] {
        assert!(text.contains(field), "{field} missing from {text}");
    }
}

#[test]
fn repeated_equal_targets_do_not_schedule_extra_animation_ticks() {
    use incular_core::Color;
    use incular_widgets::internal::RenderKind;
    let build = |color| AnimatedContainer::new(DURATION).color(color).into();
    let green = Color::rgba(0, 255, 0, 255);
    let blue = Color::rgba(0, 0, 255, 255);
    let active_tickers = |scene: &Scene| {
        let mut count = 0;
        let mut elements = vec![scene.root];
        while let Some(element) = elements.pop() {
            if let Some(RenderKind::AnimationTicker { controller, .. }) = scene
                .tree
                .render_id(element)
                .and_then(|render| scene.tree.render_object_kind(render))
            {
                count += usize::from(controller.is_active());
            }
            elements.extend(scene.tree.children(element).unwrap_or_default());
        }
        count
    };
    let mut scene = Scene::new(build(green));
    assert_eq!(active_tickers(&scene), 0);
    scene.at(1_000);
    assert_eq!(active_tickers(&scene), 0);
    scene.replace(build(green), 1_000);
    assert_eq!(active_tickers(&scene), 0);
    scene.at(2_000);
    assert_eq!(active_tickers(&scene), 0);
    scene.replace(build(blue), 2_000);
    assert_eq!(active_tickers(&scene), 1);
    scene.at(3_000);
    assert_eq!(active_tickers(&scene), 0);
    scene.replace(build(blue), 3_000);
    assert_eq!(active_tickers(&scene), 0);
}
