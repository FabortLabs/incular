//! Flutter's `OverlayPortal.overlayChildLayoutBuilder`.

use std::{cell::RefCell, rc::Rc};

use incular_config::Constraints;
use incular_core::{Color, Offset, Rect, Size, Transform};
use incular_rendering::{DisplayList, PaintCommand};
use incular_widgets::internal::WidgetTree;
use incular_widgets::{
    Align, Alignment, HitTestBehavior, MetaData, OverlayChildLayoutInfo, OverlayPortal, Positioned,
    Stack, Widget,
};

fn painted_rects(list: &DisplayList) -> Vec<(Rect, Color)> {
    let mut transforms = vec![Transform::IDENTITY];
    let mut rects = Vec::new();
    for command in list.commands() {
        match command {
            PaintCommand::PushTransform { transform } => {
                let current = *transforms.last().unwrap();
                transforms.push(current.then(*transform));
            }
            PaintCommand::PopTransform => {
                transforms.pop();
            }
            PaintCommand::Rect { rect, color } => {
                rects.push((
                    transforms.last().unwrap().transform_rect_bbox(*rect),
                    *color,
                ));
            }
            _ => {}
        }
    }
    rects
}

const MENU: Color = Color::rgba(0, 0, 255, 255);

#[test]
fn the_overlay_is_built_from_the_anchor_geometry() {
    let seen = Rc::new(RefCell::new(Vec::new()));
    let portal = {
        let seen = seen.clone();
        OverlayPortal::new(Widget::box_(Size::new(40., 20.), Color::WHITE))
            .show(true)
            .overlay_child_layout_builder(move |_, info: OverlayChildLayoutInfo| {
                seen.borrow_mut().push(info);
                let anchor = info.anchor_rect;
                Stack::new([Widget::from(
                    Positioned::new(Widget::box_(Size::new(30., 30.), MENU))
                        .left(anchor.origin.x)
                        .top(anchor.origin.y + anchor.size.height),
                )])
                .into()
            })
    };
    let mut tree = WidgetTree::new();
    tree.mount(
        MetaData::new("background")
            .behavior(HitTestBehavior::Opaque)
            .child(Align::new(Alignment::CENTER, portal))
            .into(),
    )
    .expect("mount");
    tree.layout(Constraints::tight(Size::new(200., 100.)))
        .expect("layout");
    let info = *seen.borrow().last().expect("built");
    // The anchor is centered: (200 - 40) / 2 and (100 - 20) / 2.
    assert_eq!(
        info,
        OverlayChildLayoutInfo {
            anchor_rect: Rect::from_origin_size(Offset::new(80., 40.), Size::new(40., 20.)),
            overlay_size: Size::new(200., 100.),
        }
    );
    let menu: Vec<_> = painted_rects(&tree.paint())
        .into_iter()
        .filter(|(_, color)| *color == MENU)
        .map(|(rect, _)| rect)
        .collect();
    assert_eq!(
        menu,
        [Rect::from_origin_size(
            Offset::new(80., 60.),
            Size::new(30., 30.)
        )]
    );
    // The overlay fills the view but only its content is hit.
    let menu_hit = tree.hit_test(Offset::new(90., 70.)).expect("menu hit");
    let anchor_hit = tree.hit_test(Offset::new(90., 45.)).expect("anchor hit");
    assert_ne!(menu_hit, anchor_hit);
    assert_eq!(
        tree.hit_test(Offset::new(5., 5.)),
        tree.hit_test(Offset::new(6., 6.))
    );
}
