//! Retained PageView controller changes and deferred commands before layout.
use incular_animation::Curve;
use incular_config::Constraints;
use incular_core::{Rect, Size};
use incular_runtime::Runtime;
use incular_widgets::{
    Builder, Directionality, PageController, PageView, Text, TextDirection, Widget, internal::Key,
};
use std::{cell::RefCell, rc::Rc, time::Duration};
fn show(c: Option<&PageController>) -> Widget {
    let pages = (0..3).map(|i| Widget::from(Text::new(i.to_string())).with_key(Key::Value(i)));
    let v = PageView::new(pages);
    let v = c.map_or(v.clone(), |c| v.controller(c.clone()));
    Directionality::new(TextDirection::Ltr, v).into()
}
fn frame(r: &mut Runtime) {
    r.run_frame(Constraints::tight(Size::new(800., 600.)))
        .unwrap();
}
fn mounted(c: Option<&PageController>) -> Runtime {
    let mut r = Runtime::new(show(c)).unwrap();
    frame(&mut r);
    r
}
fn replace(r: &mut Runtime, w: Widget) {
    let id = r.tree().root().unwrap();
    r.schedule_update(id, w).unwrap();
    frame(r);
}
fn bounds(r: &Runtime, k: u64) -> Option<Rect> {
    let t = r.tree();
    t.element_with_key(&Key::Value(k))
        .and_then(|e| t.element_bounds(e))
}
fn visible(r: &Runtime) -> Vec<u64> {
    (0..3)
        .filter(|&k| {
            bounds(r, k).is_some_and(|b| {
                b.origin.x < 800. && b.origin.x + b.size.width > 0. && b.size.width > 0.
            })
        })
        .collect()
}
fn drive(r: &mut Runtime, c: &PageController) -> Vec<Vec<u64>> {
    (0..3)
        .map(|p| {
            let _ = c.jump_to_page(p);
            frame(r);
            visible(r)
        })
        .collect()
}
fn assert_owned(r: &mut Runtime, c: &PageController) {
    assert_eq!(drive(r, c), [vec![0], vec![1], vec![2]]);
}

#[test]
fn adding_external_controller_to_controllerless_view_grants_page_control() {
    let mut r = mounted(None);
    let c = PageController::new();
    replace(&mut r, show(Some(&c)));
    assert_owned(&mut r, &c);
}
#[test]
fn previous_controller_loses_visual_authority_when_replaced() {
    let (a, b) = (PageController::new(), PageController::new());
    let mut r = mounted(Some(&a));
    assert_owned(&mut r, &a);
    replace(&mut r, show(Some(&b)));
    let initial = visible(&r);
    let while_detached = drive(&mut r, &a);
    assert_eq!(while_detached, [initial.clone(), initial.clone(), initial]);
    assert_owned(&mut r, &b);
}
#[test]
fn prior_external_controller_cannot_move_a_now_controllerless_view() {
    let c = PageController::new();
    let mut r = mounted(Some(&c));
    assert_owned(&mut r, &c);
    replace(&mut r, show(None));
    let initial = visible(&r);
    assert_eq!(
        drive(&mut r, &c),
        [initial.clone(), initial.clone(), initial]
    );
}
#[test]
fn controllerless_page_view_can_rebuild_with_no_external_controller_again() {
    let mut r = mounted(None);
    assert_eq!(visible(&r), [0]);
    replace(&mut r, show(None));
    assert_eq!(visible(&r), [0]);
}
#[test]
fn page_getter_is_readable_from_a_child_builder_before_content_metrics_exist() {
    let c = PageController::new();
    let values = Rc::new(RefCell::new(Vec::new()));
    let build = || {
        let v = values.clone();
        let ctrl = c.clone();
        let child = Builder::new(move |_| {
            let page = ctrl.page();
            v.borrow_mut().push(page);
            Text::new(if page.is_some() { "ready" } else { "not ready" }).into()
        });
        Directionality::new(
            TextDirection::Ltr,
            PageView::new([child]).controller(c.clone()),
        )
        .into()
    };
    let mut r = Runtime::new(build()).unwrap();
    frame(&mut r);
    let first = values.borrow().first().copied();
    replace(&mut r, build());
    let last = values.borrow().last().copied();
    assert_eq!(
        (first, last, c.page()),
        (Some(None), Some(Some(0.)), Some(0.))
    );
}
#[test]
fn jump_and_animate_calls_before_the_first_layout_are_safe_and_apply_page() {
    let actual = [false, true].map(|animated| {
        let c = PageController::new();
        let mut r = Runtime::new(show(Some(&c))).unwrap();
        let requested = if animated {
            c.animate_to_page(1, Duration::from_millis(50), Curve::EaseInBounce)
        } else {
            c.jump_to_page(1)
        };
        frame(&mut r);
        (requested, visible(&r), c.page())
    });
    assert_eq!(
        actual,
        [(true, vec![1], Some(1.)), (true, vec![1], Some(1.))]
    );
}

fn issue(c: &PageController, animated: bool) -> bool {
    if animated {
        c.animate_to_page(2, Duration::from_millis(300), Curve::Ease)
    } else {
        c.jump_to_page(2)
    }
}

#[test]
fn reject_page_commands_when_two_live_views_share_a_controller() {
    use incular_widgets::{Column, Expanded};
    let observations = [false, true].map(|animated| {
        let c = PageController::new();
        let branches = [10, 20].map(|base| {
            Expanded::new(
                PageView::new((0..3).map(|i| {
                    Widget::from(Text::new(format!("page {base}/{i}")))
                        .with_key(Key::Value(base + i))
                }))
                .controller(c.clone()),
            )
        });
        let mut r =
            Runtime::new(Directionality::new(TextDirection::Ltr, Column::new(branches)).into())
                .unwrap();
        let initial = r.run_frame(Constraints::tight(Size::new(800., 600.)));
        if matches!(
            &initial,
            Err(incular_widgets::internal::TreeError::DuplicateScrollAttachment { .. })
        ) {
            // Incular rejects a duplicate owner at attachment; Flutter
            // accepts both attachments and rejects the later command.
            return false;
        }
        initial.unwrap();
        assert_eq!(bounds(&r, 10).unwrap().size, Size::new(800., 300.));
        assert_eq!(bounds(&r, 20).unwrap().size, Size::new(800., 300.));
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| issue(&c, animated)))
            .unwrap_or(false)
    });
    assert_eq!(observations, [false, false], "jump, animate accepted");
}
#[test]
fn reject_commands_while_unattached_and_allow_the_same_actions_when_attached() {
    use incular_core::time::Instant;
    let observations = [false, true].map(|animated| {
        let c = PageController::new();
        let mut r = mounted(None);
        let accepted =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| issue(&c, animated)))
                .unwrap_or(false);
        assert_eq!(
            visible(&r),
            [0],
            "unattached controller must not move another viewport"
        );
        replace(&mut r, show(Some(&c)));
        let before = c.page();
        let _ = issue(&c, animated);
        let now = Instant::now();
        for i in 0..40 {
            r.run_frame_at(
                Constraints::tight(Size::new(800., 600.)),
                now + Duration::from_millis(i * 16),
            )
            .unwrap();
        }
        (accepted, before, c.page(), visible(&r))
    });
    assert_eq!(
        observations,
        [
            (false, Some(0.), Some(2.), vec![2]),
            (false, Some(0.), Some(2.), vec![2])
        ]
    );
}

#[test]
fn cold_default_page_view_does_not_materialize_or_report_full_offscreen_pages() {
    let pages =
        (0..3).map(|i| Widget::from(Text::new(format!("Page {i}"))).with_key(Key::Value(i)));
    let mut r =
        Runtime::new(Directionality::new(TextDirection::Ltr, PageView::new(pages)).into()).unwrap();
    frame(&mut r);
    let tree = r.tree();
    let actual = [0, 1, 2].map(|i| {
        let label = format!("Page {i}");
        (
            tree.element_with_key(&Key::Value(i)).is_some(),
            tree.semantics()
                .iter()
                .any(|(_, node)| node.label.as_deref() == Some(label.as_str())),
        )
    });
    assert_eq!(actual, [(true, true), (false, false), (false, false)]);
}
