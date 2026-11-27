use std::time::{Duration, Instant};

use incular_config::{Constraints, TextDirection};
use incular_core::{Offset, Rect, Size};
use incular_widgets::internal::{ElementId, Key, TreeError, WidgetTree};
use incular_widgets::{
    AnimatedPositioned, AnimatedPositionedDirectional, Center, Directionality, SizedBox, Stack,
    Text, Widget,
};

const DURATION: Duration = Duration::from_secs(2);
const VIEW: Size = Size {
    width: 800.,
    height: 600.,
};

fn center_of(rect: Rect) -> Offset {
    rect.origin + Offset::new(rect.size.width / 2., rect.size.height / 2.)
}

#[derive(Clone, Copy)]
enum Coordinates {
    Physical,
    Ltr,
    Rtl,
}

impl Coordinates {
    fn build(self, key: &Key, leading: f32, top: f32, trailing: Option<f32>) -> Widget {
        let child = Widget::from(SizedBox::shrink()).with_key(key.clone());
        let positioned: Widget = match self {
            Self::Physical => {
                let spec = AnimatedPositioned::new(DURATION, child)
                    .left(leading)
                    .top(top)
                    .height(100.);
                match trailing {
                    Some(trailing) => spec.right(trailing),
                    None => spec.width(100.),
                }
                .into()
            }
            Self::Ltr | Self::Rtl => {
                let spec = AnimatedPositionedDirectional::new(DURATION, child)
                    .start(leading)
                    .top(top)
                    .height(100.);
                match trailing {
                    Some(trailing) => spec.end(trailing),
                    None => spec.width(100.),
                }
                .into()
            }
        };
        let stack = Stack::new([positioned]);
        match self {
            Self::Physical => stack.text_direction(TextDirection::Ltr).into(),
            Self::Ltr => Directionality::new(TextDirection::Ltr, stack).into(),
            Self::Rtl => Directionality::new(TextDirection::Rtl, stack).into(),
        }
    }

    fn expected_center(self, from_leading: f32, y: f32) -> Offset {
        let x = match self {
            Self::Rtl => VIEW.width - from_leading,
            Self::Physical | Self::Ltr => from_leading,
        };
        Offset::new(x, y)
    }
}

struct Scene {
    tree: WidgetTree,
    root: ElementId,
    start: Instant,
}

impl Scene {
    fn mount(widget: Widget) -> Self {
        let mut tree = WidgetTree::new();
        let root = tree.mount(widget).unwrap();
        let mut out = Self {
            tree,
            root,
            start: Instant::now(),
        };
        out.frame(0);
        out.frame(0);
        out
    }

    fn frame(&mut self, millis: u64) {
        self.tree
            .update_compositor(self.start + Duration::from_millis(millis))
            .unwrap();
        self.tree.layout(Constraints::tight(VIEW)).unwrap();
    }

    fn update(&mut self, widget: Widget, millis: u64) {
        self.tree.update(self.root, widget).unwrap();
        self.frame(millis);
        self.frame(millis);
    }

    fn bounds(&self, key: &Key) -> Rect {
        self.tree
            .element_bounds(self.tree.element_with_key(key).unwrap())
            .unwrap()
    }
}

#[test]
fn from_rect_sets_its_four_fields_and_keeps_the_remaining_insets_unset() {
    let rect = Rect::from_origin_size(Offset::new(7., 5.), Size::new(12., 16.));
    let key = Key::from("from rect child");
    let child = Widget::from(SizedBox::shrink()).with_key(key.clone());
    let spec = AnimatedPositioned::from_rect(rect, Duration::from_millis(200), child);
    assert_eq!(
        (
            spec.get_left(),
            spec.get_top(),
            spec.get_width(),
            spec.get_height()
        ),
        (Some(7.), Some(5.), Some(12.), Some(16.))
    );
    assert_eq!((spec.get_right(), spec.get_bottom()), (None, None));
    let description = format!("{spec:?}");
    assert!(description.starts_with("AnimatedPositioned"));
    assert_eq!(description.trim(), description);
    assert!(!description.contains('\n'));
    let scene = Scene::mount(Stack::new([spec]).text_direction(TextDirection::Ltr).into());
    assert_eq!(scene.bounds(&key), rect);
}

fn check_interrupted(mode: Coordinates) {
    let key = Key::from("retargeted child");
    let make = |edge| mode.build(&key, edge, edge, None);
    let mut scene = Scene::mount(make(0.));
    let mounted = scene.tree.element_with_key(&key).unwrap();
    let verify = |scene: &Scene, center| {
        assert_eq!(scene.tree.element_with_key(&key), Some(mounted));
        assert_eq!(scene.bounds(&key).size, Size::new(100., 100.));
        assert_eq!(
            center_of(scene.bounds(&key)),
            mode.expected_center(center, center)
        );
    };
    verify(&scene, 50.);
    scene.frame(1_000);
    verify(&scene, 50.);
    scene.update(make(100.), 1_000);
    verify(&scene, 50.);
    scene.frame(2_000);
    verify(&scene, 100.);
    scene.update(make(150.), 2_000);
    verify(&scene, 100.);
    scene.frame(3_000);
    verify(&scene, 150.);
    scene.frame(4_000);
    verify(&scene, 200.);
}

#[test]
fn physically_positioned_child_can_retarget_midflight() {
    check_interrupted(Coordinates::Physical);
}

#[test]
fn ltr_directional_child_can_retarget_midflight() {
    check_interrupted(Coordinates::Ltr);
}

#[test]
fn rtl_directional_child_can_retarget_midflight() {
    check_interrupted(Coordinates::Rtl);
}

fn check_switch(mode: Coordinates) {
    let key = Key::from("switch from width to trailing edge");
    let mut scene = Scene::mount(mode.build(&key, 0., 0., None));
    let verify = |scene: &Scene, leading_center, y, width| {
        let rect = scene.bounds(&key);
        assert_eq!(center_of(rect), mode.expected_center(leading_center, y));
        assert_eq!(rect.size, Size::new(width, 100.));
    };
    verify(&scene, 50., 50., 100.);
    scene.frame(1_000);
    verify(&scene, 50., 50., 100.);
    scene.update(mode.build(&key, 0., 100., Some(100.)), 1_000);
    verify(&scene, 350., 50., 700.);
    scene.frame(2_000);
    verify(&scene, 350., 100., 700.);
    scene.frame(3_000);
    verify(&scene, 350., 150., 700.);
}

#[test]
fn physical_trailing_edge_replaces_width_immediately_while_top_animates() {
    check_switch(Coordinates::Physical);
}

#[test]
fn ltr_trailing_edge_replaces_width_immediately_while_top_animates() {
    check_switch(Coordinates::Ltr);
}

#[test]
fn rtl_trailing_edge_replaces_width_immediately_while_top_animates() {
    check_switch(Coordinates::Rtl);
}

#[test]
fn directional_child_without_edges_can_settle_at_zero_size() {
    let key = Key::from("zero animated directional area");
    let animated = Widget::from(AnimatedPositionedDirectional::new(
        Duration::from_millis(300),
        Text::new("X"),
    ))
    .with_key(key.clone());
    let mut scene = Scene::mount(
        Directionality::new(
            TextDirection::Ltr,
            Center::new(SizedBox::shrink().child(Stack::new([animated]))),
        )
        .into(),
    );
    scene.frame(500);
    assert_eq!(scene.bounds(&key).size, Size::ZERO);
}

#[test]
fn explicit_stack_direction_does_not_replace_an_inherited_direction() {
    let animated =
        AnimatedPositionedDirectional::new(Duration::from_millis(200), SizedBox::shrink())
            .start(10.);
    let mut tree = WidgetTree::new();
    let stack = Stack::new([animated])
        .text_direction(TextDirection::Ltr)
        .into();
    let result = tree
        .mount(stack)
        .and_then(|_| tree.layout(Constraints::tight(VIEW)));
    assert!(
        matches!(
            result,
            Err(TreeError::MissingDirectionality {
                widget: "AnimatedPositionedDirectional",
                ..
            })
        ),
        "{result:?}"
    );
}

#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "only two horizontal positioning values")]
fn physical_horizontal_edges_and_width_cannot_all_be_set() {
    let _ = Widget::from(
        AnimatedPositioned::new(Duration::ZERO, SizedBox::shrink())
            .left(1.)
            .right(2.)
            .width(3.),
    );
}

#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "only two vertical positioning values")]
fn directional_vertical_edges_and_height_cannot_all_be_set() {
    let _ = Widget::from(
        AnimatedPositionedDirectional::new(Duration::ZERO, SizedBox::shrink())
            .top(1.)
            .bottom(2.)
            .height(3.),
    );
}
