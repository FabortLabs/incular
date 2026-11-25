//! Pinned PageView restoration, controller attachment, and default accessibility.
use incular_config::Constraints;
use incular_core::{Size, time::Instant};
use incular_runtime::Runtime;
use incular_widgets::{
    Container, Directionality, PageController, PageStorage, PageStorageBucket, PageStorageKey,
    PageView, Placeholder, ScrollController, Semantics, Text, TextDirection, Widget,
    internal::page_for_pixels,
};
use std::time::Duration;

fn constraint() -> Constraints {
    Constraints::tight(Size::new(800., 600.))
}
fn frame(r: &mut Runtime) {
    r.run_frame(constraint()).unwrap();
}
fn settle(r: &mut Runtime) {
    let base = Instant::now();
    for step in 0..120 {
        r.run_frame_at(constraint(), base + Duration::from_millis(16 * step))
            .unwrap();
    }
}
fn replace(r: &mut Runtime, w: Widget) {
    let id = r.tree().root().unwrap();
    r.schedule_update(id, w).unwrap();
    frame(r);
}

#[test]
fn page_storage_detaches_and_restores_the_selected_page() {
    let c = PageController::new();
    assert_eq!(c.page(), None);
    let bucket = PageStorageBucket::default();
    let build = |controller: &PageController, key: &str| {
        let pages = (0..3).map(|_| Placeholder::new());
        let view = PageView::new(pages)
            .controller(controller.clone())
            .page_storage_key(PageStorageKey::new(key));
        Directionality::new(
            TextDirection::Ltr,
            PageStorage::with_bucket(bucket.clone(), view),
        )
        .into()
    };
    let mut r = Runtime::new(build(&c, "page")).unwrap();
    frame(&mut r);
    let initial = c.page();
    assert!(c.jump_to_page(2));
    settle(&mut r);
    let selected = c.page();
    replace(
        &mut r,
        Directionality::new(
            TextDirection::Ltr,
            PageStorage::with_bucket(bucket.clone(), Container::new()),
        )
        .into(),
    );
    let detached = c.page();
    replace(&mut r, build(&c, "page"));
    let restored = c.page();
    let off = PageController::from(ScrollController::new().with_keep_scroll_offset(false));
    replace(&mut r, build(&off, "fresh page"));
    assert_eq!(
        (initial, selected, detached, restored, off.page()),
        (Some(0.), Some(2.), None, Some(2.), Some(0.))
    );
}

#[test]
fn fresh_controller_reads_storage_instead_of_reusing_the_source_controller_offset() {
    let bucket = PageStorageBucket::default();
    let build = |c: &PageController| {
        Directionality::new(
            TextDirection::Ltr,
            PageStorage::with_bucket(
                bucket.clone(),
                PageView::new([Placeholder::new(), Placeholder::new(), Placeholder::new()])
                    .controller(c.clone())
                    .page_storage_key(PageStorageKey::new("shared page")),
            ),
        )
        .into()
    };
    let original = PageController::new();
    let mut r = Runtime::new(build(&original)).unwrap();
    frame(&mut r);
    assert!(original.jump_to_page(2));
    settle(&mut r);
    assert_eq!(original.page(), Some(2.));
    replace(
        &mut r,
        Directionality::new(
            TextDirection::Ltr,
            PageStorage::with_bucket(bucket.clone(), Container::new()),
        )
        .into(),
    );
    let fresh = PageController::new();
    assert_eq!(fresh.page(), None);
    replace(&mut r, build(&fresh));
    assert_eq!(fresh.page(), Some(2.));
}
#[test]
fn only_the_current_full_size_page_contributes_its_label_to_semantics() {
    let c = PageController::new();
    let pages = (0..3).map(|i| Semantics::new(Text::new(format!("Page #{i}"))).container(true));
    let mut r = Runtime::new(
        Directionality::new(
            TextDirection::Ltr,
            PageView::new(pages).controller(c.clone()),
        )
        .into(),
    )
    .unwrap();
    frame(&mut r);
    assert_eq!(c.page(), Some(0.));
    let actual = [0, 1, 2].map(|page| {
        if page != 0 {
            assert!(c.jump_to_page(page));
            settle(&mut r);
        }
        [0, 1, 2].map(|i| {
            let name = format!("Page #{i}");
            r.tree()
                .semantics()
                .iter()
                .any(|(_, n)| n.label.as_deref() == Some(name.as_str()))
        })
    });
    assert_eq!(
        actual,
        [
            [true, false, false],
            [false, true, false],
            [false, false, true]
        ]
    );
}
#[test]
fn page_metric_calculation_clamps_pixels_before_converting() {
    let page = |pixels: f32| page_for_pixels(pixels.clamp(100., 200.), 25., 1.);
    assert_eq!([page(150.), page(50.)], [6., 4.]);
}
#[test]
fn controller_resolves_the_reference_rounding_input_to_page_one() {
    let c = PageController::new();
    let input = 799.99999999999f64 as f32;
    let pages = (0..3).map(|i| Semantics::new(Text::new(format!("Page #{i}"))).container(true));
    let mut r = Runtime::new(
        Directionality::new(
            TextDirection::Ltr,
            PageView::new(pages).controller(c.clone()),
        )
        .into(),
    )
    .unwrap();
    frame(&mut r);
    assert!(c.jump_to(input));
    assert_eq!(c.page(), Some(1.));
}

#[test]
fn storage_restores_the_logical_page_after_width_and_fraction_change() {
    use incular_widgets::{Center, SizedBox};

    let bucket = PageStorageBucket::default();
    let key = PageStorageKey::new("logical page");
    let build = |controller: Option<&PageController>, width| {
        let body = if let Some(controller) = controller {
            Widget::from(
                PageView::new((0..5).map(|_| Placeholder::new()))
                    .controller(controller.clone())
                    .page_storage_key(key.clone()),
            )
        } else {
            Widget::from(Container::new())
        };
        Directionality::new(
            TextDirection::Ltr,
            PageStorage::with_bucket(
                bucket.clone(),
                Center::new(SizedBox::from_size(Size::new(width, 300.)).child(body)),
            ),
        )
        .into()
    };

    let first = PageController::new().with_viewport_fraction(0.75);
    let mut runtime = Runtime::new(build(Some(&first), 400.)).unwrap();
    frame(&mut runtime);
    assert!(first.jump_to_page(2));
    frame(&mut runtime);
    assert_eq!(first.offset(), 600.);
    assert_eq!(bucket.read_state(&key).unwrap().as_f64(), Some(2.));

    replace(&mut runtime, build(None, 400.));
    assert_eq!(first.page(), None);
    let restored = PageController::new()
        .with_initial_page(1)
        .with_viewport_fraction(0.5);
    replace(&mut runtime, build(Some(&restored), 500.));
    assert_eq!((restored.page(), restored.offset()), (Some(2.), 500.));

    replace(&mut runtime, build(None, 500.));
    let disabled = PageController::from(ScrollController::new().with_keep_scroll_offset(false))
        .with_initial_page(1)
        .with_viewport_fraction(0.5);
    replace(&mut runtime, build(Some(&disabled), 500.));
    assert_eq!((disabled.page(), disabled.offset()), (Some(1.), 250.));
    assert!(disabled.jump_to_page(3));
    frame(&mut runtime);
    assert_eq!(bucket.read_state(&key).unwrap().as_f64(), Some(2.));

    // Incular's explicit pre-layout command is an extension over Flutter.
    replace(&mut runtime, build(None, 500.));
    let queued = PageController::new().with_viewport_fraction(0.5);
    assert!(queued.jump_to_page(3));
    replace(&mut runtime, build(Some(&queued), 500.));
    assert_eq!((queued.page(), queued.offset()), (Some(3.), 750.));
}

#[test]
fn animated_accessibility_pages_expose_only_the_current_and_hidden_neighbors() {
    let controller = PageController::new();
    let pages = (0..4).map(|i| Semantics::new(Text::new(format!("Page #{i}"))).container(true));
    let mut runtime = Runtime::new(
        Directionality::new(
            TextDirection::Ltr,
            PageView::new(pages)
                .controller(controller.clone())
                .allow_implicit_scrolling(true),
        )
        .into(),
    )
    .unwrap();
    let mut now = Instant::now();
    let mut pump = |runtime: &mut Runtime| {
        runtime.run_frame_at(constraint(), now).unwrap();
        now += Duration::from_millis(16);
    };
    pump(&mut runtime);
    assert!(
        runtime
            .tree()
            .semantics()
            .iter()
            .any(|(_, node)| node.state.has_implicit_scrolling)
    );

    let cached_visibility = |runtime: &Runtime| {
        ["Page #0", "Page #1", "Page #2", "Page #3"].map(|label| {
            runtime
                .tree()
                .semantics()
                .iter()
                .find(|(_, node)| node.label.as_deref() == Some(label))
                .map(|(_, node)| node.state.hidden)
        })
    };
    assert_eq!(controller.page(), Some(0.));
    assert_eq!(
        cached_visibility(&runtime),
        [Some(false), Some(true), None, None]
    );
    for (page, expected) in [
        (1., [Some(true), Some(false), Some(true), None]),
        (2., [None, Some(true), Some(false), Some(true)]),
    ] {
        assert!(controller.next_page(Duration::from_millis(150), incular_animation::Curve::Ease));
        for _ in 0..32 {
            pump(&mut runtime);
        }
        assert_eq!(controller.page(), Some(page));
        assert_eq!(cached_visibility(&runtime), expected);
    }
}
