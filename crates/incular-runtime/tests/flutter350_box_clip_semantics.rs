use std::{cell::Cell, rc::Rc};

use incular_config::{Clip, Constraints, TextDirection};
use incular_core::{Offset, Rect, Size};
use incular_runtime::Runtime;
use incular_widgets::{
    ClipReclip, ClipRect, ClipShape, CustomClipper, Flex, SizedBox, Text, Widget,
};

#[test]
fn approximate_box_clip_disappears_only_for_none() {
    let bounds = Size::new(200., 200.);
    let make = |mode| Widget::from(ClipRect::new(SizedBox::from_size(bounds)).clip_behavior(mode));
    let mut runtime = Runtime::new(make(Clip::None)).unwrap();
    let root = runtime.tree().root().unwrap();
    let render = runtime.tree().render_id(root).unwrap();

    for mode in [
        Clip::None,
        Clip::HardEdge,
        Clip::AntiAlias,
        Clip::AntiAliasWithSaveLayer,
        Clip::None,
    ] {
        runtime.schedule_update(root, make(mode)).unwrap();
        runtime.run_frame(Constraints::tight(bounds)).unwrap();
        let tree = runtime.tree();
        assert_eq!(tree.render_id(root), Some(render));
        assert_eq!(
            tree.describe_approximate_paint_clip(render),
            (mode != Clip::None).then_some(Rect::from_origin_size(Offset::ZERO, bounds))
        );
    }
}

struct AccessibilityClip {
    width: Rc<Cell<f32>>,
    reclip: ClipReclip,
}

impl CustomClipper for AccessibilityClip {
    fn get_clip(&self, size: Size) -> ClipShape {
        ClipShape::Rect(Rect::from_origin_size(Offset::ZERO, size))
    }

    fn get_approximate_clip_rect(&self, size: Size) -> Rect {
        Rect::from_origin_size(Offset::ZERO, Size::new(self.width.get(), size.height))
    }

    fn reclip(&self) -> Option<ClipReclip> {
        Some(self.reclip.clone())
    }
}

#[test]
fn a_live_accessibility_clip_can_change_semantic_bounds_without_layout_or_hit_changes() {
    let width = Rc::new(Cell::new(100.));
    let reclip = ClipReclip::default();
    let clipped = ClipRect::new(
        Flex::horizontal(["1", "2", "3"].map(|label| {
            SizedBox::new()
                .width(75.)
                .height(14.)
                .child(Text::new(label).text_direction(TextDirection::Ltr))
        }))
        .text_direction(TextDirection::Ltr),
    )
    .custom_clipper(AccessibilityClip {
        width: width.clone(),
        reclip: reclip.clone(),
    });
    let mut runtime = Runtime::new(clipped.into()).unwrap();
    let view = Constraints::tight(Size::new(225., 14.));
    runtime.run_frame(view).unwrap();
    let render = runtime
        .tree()
        .render_id(runtime.tree().root().unwrap())
        .unwrap();
    let last_point = Offset::new(200., 7.);
    let hit = runtime
        .tree()
        .hit_test(last_point)
        .expect("third painted label is hittable");
    let layouts = runtime.diagnostics().layouts;
    let observed = |runtime: &Runtime| {
        runtime
            .tree()
            .semantics()
            .iter()
            .filter_map(|(_, node)| node.label.as_ref().map(|_| node.bounds.size.width))
            .collect::<Vec<_>>()
    };
    assert_eq!(observed(&runtime), [75., 25.]);

    for (width_px, expected) in [
        (140., vec![75., 65.]),
        (225., vec![75., 75., 75.]),
        (100., vec![75., 25.]),
    ] {
        width.set(width_px);
        reclip.notify();
        assert!(runtime.frame_requested());
        runtime.run_frame(view).unwrap();
        assert_eq!(runtime.diagnostics().layouts, layouts);
        assert_eq!(runtime.tree().hit_test(last_point), Some(hit));
        assert_eq!(
            runtime
                .tree()
                .describe_approximate_paint_clip(render)
                .unwrap()
                .size
                .width,
            width_px
        );
        assert_eq!(observed(&runtime), expected);
    }
}
