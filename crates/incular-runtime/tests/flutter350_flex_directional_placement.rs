use incular_config::{
    Axis, Constraints, CrossAxisAlignment, MainAxisAlignment, TextDirection, VerticalDirection,
};
use incular_core::{Offset, Rect, Size};
use incular_runtime::Runtime;
use incular_widgets::{Flex, SizedBox, Widget, internal::Key};

#[derive(Clone, Copy)]
struct Policy {
    axis: Axis,
    main: MainAxisAlignment,
    cross: CrossAxisAlignment,
    text: TextDirection,
    vertical: VerticalDirection,
}

impl Default for Policy {
    fn default() -> Self {
        Self {
            axis: Axis::Horizontal,
            main: MainAxisAlignment::Start,
            cross: CrossAxisAlignment::Center,
            text: TextDirection::Ltr,
            vertical: VerticalDirection::Down,
        }
    }
}

impl Policy {
    fn widget(self, children: usize) -> Widget {
        Flex::new(
            self.axis,
            (0..children).map(|i| {
                Widget::from(SizedBox::from_size(Size::new(100., 100.)))
                    .with_key(Key::Value(i as u64))
            }),
        )
        .main_axis_alignment(self.main)
        .cross_axis_alignment(self.cross)
        .text_direction(self.text)
        .vertical_direction(self.vertical)
        .into()
    }
}

fn frame(runtime: &mut Runtime) {
    runtime
        .run_frame(Constraints::tight(Size::new(800., 600.)))
        .unwrap();
}

#[test]
fn the_same_flex_children_follow_every_pinned_direction_change() {
    let mut policy = Policy::default();
    let mut runtime = Runtime::new(policy.widget(3)).unwrap();
    frame(&mut runtime);
    let root = runtime.tree().root().unwrap();
    let children = [0, 1, 2].map(|key| runtime.tree().element_with_key(&Key::Value(key)).unwrap());

    let mut check = |policy: Policy, x: [f32; 3], y: [f32; 3], width: f32| {
        runtime.schedule_update(root, policy.widget(3)).unwrap();
        frame(&mut runtime);
        for (index, child) in children.iter().enumerate() {
            let tree = runtime.tree();
            assert_eq!(
                tree.element_with_key(&Key::Value(index as u64)),
                Some(*child)
            );
            assert_eq!(
                tree.element_bounds(*child),
                Some(Rect::from_origin_size(
                    Offset::new(x[index], y[index]),
                    Size::new(width, 100.)
                )),
                "child {index}"
            );
        }
    };

    use CrossAxisAlignment::{End as CrossEnd, Start as CrossStart, Stretch};
    use MainAxisAlignment::{End, Start};
    let start = [0., 100., 200.];
    let end = [500., 600., 700.];
    let reverse_end = [700., 600., 500.];
    let vertical_up = [500., 400., 300.];
    check(policy, start, [250.; 3], 100.);
    policy.main = End;
    check(policy, end, [250.; 3], 100.);
    policy.text = TextDirection::Rtl;
    check(policy, [200., 100., 0.], [250.; 3], 100.);
    policy.main = Start;
    check(policy, reverse_end, [250.; 3], 100.);
    policy.cross = CrossStart;
    check(policy, reverse_end, [0.; 3], 100.);
    policy.cross = CrossEnd;
    check(policy, reverse_end, [500.; 3], 100.);
    policy.vertical = VerticalDirection::Up;
    check(policy, reverse_end, [0.; 3], 100.);
    policy.cross = CrossStart;
    check(policy, reverse_end, [500.; 3], 100.);
    policy.axis = Axis::Vertical;
    check(policy, [700.; 3], vertical_up, 100.);
    policy.cross = CrossEnd;
    check(policy, [0.; 3], vertical_up, 100.);
    policy.cross = Stretch;
    check(policy, [0.; 3], vertical_up, 800.);
    policy.text = TextDirection::Ltr;
    check(policy, [0.; 3], vertical_up, 800.);
    policy.cross = CrossStart;
    check(policy, [0.; 3], vertical_up, 100.);
    policy.cross = CrossEnd;
    check(policy, [700.; 3], vertical_up, 100.);
    policy.vertical = VerticalDirection::Down;
    check(policy, [700.; 3], start, 100.);
    policy.main = End;
    check(policy, [700.; 3], [300., 400., 500.], 100.);
}

#[test]
fn children_without_baselines_are_at_the_top_in_either_vertical_direction() {
    let mut policy = Policy {
        cross: CrossAxisAlignment::Baseline,
        vertical: VerticalDirection::Up,
        ..Policy::default()
    };
    let mut runtime = Runtime::new(policy.widget(2)).unwrap();
    frame(&mut runtime);
    let root = runtime.tree().root().unwrap();

    for vertical in [VerticalDirection::Up, VerticalDirection::Down] {
        policy.vertical = vertical;
        runtime.schedule_update(root, policy.widget(2)).unwrap();
        frame(&mut runtime);
        for key in [0, 1] {
            let child = runtime.tree().element_with_key(&Key::Value(key)).unwrap();
            assert_eq!(runtime.tree().element_bounds(child).unwrap().origin.y, 0.);
        }
    }
}
