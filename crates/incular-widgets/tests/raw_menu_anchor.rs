//! Flutter's `RawMenuAnchor`, `RawMenuAnchorGroup` and `MenuController`.

use std::{cell::Cell, rc::Rc, time::Instant};

use incular_config::{Constraints, TextDirection};
use incular_core::{Color, Offset, PRIMARY_POINTER_BUTTON, PointerPhase, Rect, Size, Transform};
use incular_gestures::{PointerDeviceKind, RawPointerEvent};
use incular_rendering::{DisplayList, PaintCommand};
use incular_widgets::internal::WidgetTree;
use incular_widgets::{
    Align, Alignment, Directionality, FocusNode, MenuController, Positioned, RawMenuAnchor,
    RawMenuAnchorGroup, RawMenuOverlayInfo, Row, SizedBox, Stack, TapRegion, TapRegionSurface,
    Widget,
};

const MENU: Color = Color::rgba(0, 0, 255, 255);
const SUBMENU: Color = Color::rgba(0, 255, 0, 255);

fn painted(list: &DisplayList, color: Color) -> Vec<Rect> {
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
            PaintCommand::Rect { rect, color: c } if *c == color => {
                rects.push(transforms.last().unwrap().transform_rect_bbox(*rect));
            }
            _ => {}
        }
    }
    rects
}

/// A menu below its anchor: a `size` square of `color` inside the menu's
/// tap region, holding `content`.
fn menu_below(info: &RawMenuOverlayInfo, size: f32, color: Color, content: Widget) -> Widget {
    let anchor = info.anchor_rect;
    let position = info.position.unwrap_or(Offset::new(0., anchor.size.height));
    Stack::new([Widget::from(
        Positioned::new(
            TapRegion::new(Stack::new([
                Widget::box_(Size::new(size, size), color),
                content,
            ]))
            .group_id(info.tap_region_group_id.clone()),
        )
        .left(anchor.origin.x + position.x)
        .top(anchor.origin.y + position.y),
    )])
    .into()
}

fn anchor_box() -> Widget {
    Widget::box_(Size::new(40., 20.), Color::WHITE)
}

struct Harness {
    tree: WidgetTree,
}

impl Harness {
    fn new(widget: Widget) -> Self {
        let mut tree = WidgetTree::new();
        tree.mount(
            Directionality::new(
                TextDirection::Ltr,
                TapRegionSurface::new(Align::new(Alignment::TOP_LEFT, widget)),
            )
            .into(),
        )
        .expect("mount");
        let mut harness = Self { tree };
        harness.frame();
        harness
    }

    fn frame(&mut self) {
        self.tree
            .layout(Constraints::tight(Size::new(200., 200.)))
            .expect("layout");
    }

    fn tap(&mut self, point: Offset) {
        let event = RawPointerEvent {
            buttons: PRIMARY_POINTER_BUTTON,
            button: Some(PRIMARY_POINTER_BUTTON),
            ..RawPointerEvent::new(
                0,
                1,
                PointerDeviceKind::Mouse,
                0,
                point,
                PointerPhase::Down,
                Instant::now(),
            )
        };
        let _ = self.tree.dispatch_raw_pointer_in_window(1, event);
        self.frame();
    }

    fn menus(&mut self, color: Color) -> Vec<Rect> {
        painted(&self.tree.paint(), color)
    }
}

#[test]
fn the_controller_opens_the_overlay_where_the_builder_puts_it() {
    let controller = MenuController::new();
    let (opened, closed) = (Rc::new(Cell::new(0)), Rc::new(Cell::new(0)));
    // The panel closes the menu on a tap outside the tree, as Flutter's
    // `overlayBuilder` documentation shows.
    let anchor = RawMenuAnchor::new(controller.clone(), |context, info| {
        let menu = MenuController::maybe_of(context).expect("the overlay sees its menu");
        Stack::new([Widget::from(
            TapRegion::new(menu_below(info, 50., MENU, SizedBox::shrink().into()))
                .group_id(info.tap_region_group_id.clone())
                .on_tap_outside(move |_| menu.close()),
        )])
        .into()
    })
    .child(anchor_box())
    .on_open({
        let opened = opened.clone();
        move || opened.set(opened.get() + 1)
    })
    .on_close({
        let closed = closed.clone();
        move || closed.set(closed.get() + 1)
    });
    let mut harness = Harness::new(anchor.into());
    assert!(!controller.is_open());
    assert!(harness.menus(MENU).is_empty());

    controller.open(None);
    harness.frame();
    assert!(controller.is_open());
    assert_eq!(opened.get(), 1);
    assert_eq!(
        harness.menus(MENU),
        [Rect::from_origin_size(
            Offset::new(0., 20.),
            Size::new(50., 50.)
        )]
    );
    // Opening an open menu at a position closes it and opens it there.
    controller.open(Some(Offset::new(30., 5.)));
    harness.frame();
    assert_eq!((opened.get(), closed.get()), (2, 1));
    assert_eq!(harness.menus(MENU)[0].origin, Offset::new(30., 5.));

    // Taps on the menu or the anchor are inside; others close the menu.
    harness.tap(Offset::new(40., 20.));
    harness.tap(Offset::new(5., 5.));
    assert!(controller.is_open());
    harness.tap(Offset::new(150., 150.));
    assert!(!controller.is_open());
    assert_eq!(closed.get(), 2);
    assert!(harness.menus(MENU).is_empty());
}

#[test]
fn submenus_close_with_their_menu_and_with_their_siblings() {
    let root = MenuController::new();
    let (first, second) = (MenuController::new(), MenuController::new());
    let submenu = |controller: &MenuController, x: f32| -> Widget {
        Positioned::new(
            RawMenuAnchor::new(controller.clone(), |_, info| {
                menu_below(info, 10., SUBMENU, SizedBox::shrink().into())
            })
            .child(Widget::box_(Size::new(10., 10.), Color::BLACK)),
        )
        .left(x)
        .top(0.)
        .into()
    };
    let items = Stack::new([submenu(&first, 0.), submenu(&second, 20.)]);
    let items: Widget = items.into();
    let anchor = RawMenuAnchor::new(root.clone(), move |_, info| {
        menu_below(info, 50., MENU, items.clone())
    })
    .child(anchor_box());
    let mut harness = Harness::new(anchor.into());
    root.open(None);
    harness.frame();
    first.open(None);
    harness.frame();
    assert!(first.is_open());
    // Opening a sibling closes the first.
    second.open(None);
    harness.frame();
    assert!(!first.is_open() && second.is_open());
    assert_eq!(harness.menus(SUBMENU).len(), 1);
    // A tap in the submenu is inside the tree.
    let submenu = harness.menus(SUBMENU)[0];
    harness.tap(Offset::new(submenu.origin.x + 5., submenu.origin.y + 5.));
    assert!(root.is_open() && second.is_open());
    // Closing the root closes the submenu.
    root.close();
    harness.frame();
    assert!(!root.is_open() && !second.is_open());
    assert!(harness.menus(SUBMENU).is_empty());
}

#[test]
fn a_group_keeps_one_menu_open_and_closes_them_all() {
    let group = MenuController::new();
    let (left, right) = (MenuController::new(), MenuController::new());
    let anchor = |controller: &MenuController| -> Widget {
        RawMenuAnchor::new(controller.clone(), |_, info| {
            menu_below(info, 30., MENU, SizedBox::shrink().into())
        })
        .child(anchor_box())
        .into()
    };
    let mut harness = Harness::new(
        RawMenuAnchorGroup::new(group.clone(), Row::new([anchor(&left), anchor(&right)])).into(),
    );
    left.open(None);
    harness.frame();
    assert!(group.is_open());
    right.open(None);
    harness.frame();
    assert!(!left.is_open() && right.is_open());
    // The group cannot open itself, and a tap outside closes its menus.
    group.open(None);
    harness.tap(Offset::new(150., 150.));
    assert!(!group.is_open() && !right.is_open());
    left.open(None);
    harness.frame();
    group.close();
    harness.frame();
    assert!(!left.is_open());
}

#[test]
fn open_and_close_requests_can_defer_the_overlay() {
    let controller = MenuController::new();
    let show = Rc::new(std::cell::RefCell::new(None::<Rc<dyn Fn()>>));
    let hide = Rc::new(std::cell::RefCell::new(None::<Rc<dyn Fn()>>));
    let anchor = RawMenuAnchor::new(controller.clone(), |_, info| {
        menu_below(info, 50., MENU, SizedBox::shrink().into())
    })
    .child(anchor_box())
    .on_open_requested({
        let show = show.clone();
        move |_, callback| *show.borrow_mut() = Some(callback)
    })
    .on_close_requested({
        let hide = hide.clone();
        move |callback| *hide.borrow_mut() = Some(callback)
    });
    let mut harness = Harness::new(anchor.into());
    controller.open(None);
    harness.frame();
    assert!(!controller.is_open());
    (show.borrow().clone().expect("requested"))();
    harness.frame();
    assert!(controller.is_open());
    controller.close();
    assert!(controller.is_open());
    (hide.borrow().clone().expect("requested"))();
    harness.frame();
    assert!(!controller.is_open());
}

#[test]
fn an_outside_tap_closes_only_the_submenus_unless_the_panel_closes_the_menu() {
    let (menu, submenu) = (MenuController::new(), MenuController::new());
    let items: Widget = RawMenuAnchor::new(submenu.clone(), |_, info| {
        menu_below(info, 10., SUBMENU, SizedBox::shrink().into())
    })
    .child(Widget::box_(Size::new(10., 10.), Color::WHITE))
    .into();
    let anchor = RawMenuAnchor::new(menu.clone(), move |_, info| {
        menu_below(info, 50., MENU, items.clone())
    })
    .child(anchor_box());
    let mut harness = Harness::new(anchor.into());
    menu.open(None);
    harness.frame();
    submenu.open(None);
    harness.frame();
    assert!(submenu.is_open());
    // `handleOutsideTap`: the anchors close their submenus, and the panel
    // here has no region that closes the menu.
    harness.tap(Offset::new(150., 150.));
    assert!(menu.is_open() && !submenu.is_open());
}

#[test]
fn a_root_menu_focuses_its_child_focus_node_on_opening() {
    let (menu, submenu) = (MenuController::new(), MenuController::new());
    let (menu_focus, submenu_focus) = (FocusNode::new(), FocusNode::new());
    let items: Widget = RawMenuAnchor::new(submenu.clone(), |_, info| {
        menu_below(info, 10., SUBMENU, SizedBox::shrink().into())
    })
    .child_focus_node(submenu_focus.clone())
    .child(Widget::box_(Size::new(10., 10.), Color::WHITE))
    .into();
    let anchor = RawMenuAnchor::new(menu.clone(), move |_, info| {
        menu_below(info, 50., MENU, items.clone())
    })
    .child_focus_node(menu_focus.clone())
    .child(anchor_box());
    let mut harness = Harness::new(anchor.into());
    assert!(!menu_focus.has_focus());
    menu.open(None);
    harness.frame();
    assert!(menu_focus.has_focus());
    // A submenu's anchor is in its parent's overlay, which keeps focus.
    submenu.open(None);
    harness.frame();
    assert!(submenu.is_open() && !submenu_focus.has_focus());
}

#[test]
fn a_menu_directly_in_a_group_focuses_its_child_focus_node() {
    let (group, menu) = (MenuController::new(), MenuController::new());
    let focus = FocusNode::new();
    let anchor: Widget = RawMenuAnchor::new(menu.clone(), |_, info| {
        menu_below(info, 30., MENU, SizedBox::shrink().into())
    })
    .child_focus_node(focus.clone())
    .child(anchor_box())
    .into();
    let mut harness = Harness::new(RawMenuAnchorGroup::new(group, anchor).into());
    menu.open(None);
    harness.frame();
    assert!(focus.has_focus());
}

#[test]
fn a_view_size_change_closes_the_open_menus() {
    let controller = MenuController::new();
    let anchor = RawMenuAnchor::new(controller.clone(), |_, info| {
        menu_below(info, 50., MENU, SizedBox::shrink().into())
    })
    .child(anchor_box());
    let mut harness = Harness::new(anchor.into());
    let mut environment = harness.tree.environment().clone();
    environment.viewport = Size::new(200., 200.);
    let _ = harness.tree.set_environment(environment.clone());
    harness.frame();
    controller.open(None);
    harness.frame();
    // An unchanged size keeps the menu open.
    let _ = harness.tree.set_environment(environment.clone());
    harness.frame();
    assert!(controller.is_open());
    environment.viewport = Size::new(300., 200.);
    let _ = harness.tree.set_environment(environment);
    harness.frame();
    assert!(!controller.is_open());
}
