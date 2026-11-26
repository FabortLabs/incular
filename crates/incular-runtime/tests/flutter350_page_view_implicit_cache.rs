use incular_config::{Constraints, TextDirection};
use incular_core::{Size, time::Instant};
use incular_runtime::Runtime;
use incular_semantics::{SemanticAction, SemanticActionKind};
use incular_widgets::{
    Directionality, PageController, PageView, ScrollCacheExtent, Semantics, SizedBox, Widget,
    internal::Key,
};
use std::time::Duration;

const LABELS: [&str; 4] = ["Page 0", "Page 1", "Page 2", "Page 3"];

fn build(controller: &PageController, allow: bool, cache: Option<ScrollCacheExtent>) -> Widget {
    let mut view = PageView::new(LABELS.iter().enumerate().map(|(index, label)| {
        Widget::from(
            Semantics::new(SizedBox::expand())
                .label(*label)
                .button(true),
        )
        .with_key(Key::Value(index as u64))
    }))
    .controller(controller.clone())
    .allow_implicit_scrolling(allow)
    .pad_ends(false);
    if let Some(cache) = cache {
        view = view.scroll_cache_extent(cache);
    }
    Directionality::new(TextDirection::Ltr, view).into()
}

struct Host {
    runtime: Runtime,
    now: Instant,
    width: f32,
}

impl Host {
    fn new(widget: Widget) -> Self {
        let mut host = Self {
            runtime: Runtime::new(widget).unwrap(),
            now: Instant::now(),
            width: 400.,
        };
        host.frame();
        host
    }

    fn frame(&mut self) {
        self.runtime
            .run_frame_at(Constraints::tight(Size::new(self.width, 200.)), self.now)
            .unwrap();
        self.now += Duration::from_millis(16);
    }

    fn update(&mut self, widget: Widget) {
        let root = self.runtime.tree().root().unwrap();
        self.runtime.schedule_update(root, widget).unwrap();
        self.frame();
    }

    fn mounted(&self) -> Vec<usize> {
        (0..LABELS.len())
            .filter(|&i| {
                self.runtime
                    .tree()
                    .element_with_key(&Key::Value(i as u64))
                    .is_some()
            })
            .collect()
    }

    fn semantic_labels(&self) -> Vec<&str> {
        let mut labels: Vec<_> = self
            .runtime
            .tree()
            .semantics()
            .iter()
            .filter_map(|(_, node)| node.label.as_deref())
            .filter(|label| LABELS.contains(label))
            .collect();
        labels.sort_unstable();
        labels
    }

    fn reveal(&mut self, page: usize) -> bool {
        let (id, node) = self
            .runtime
            .tree()
            .semantics()
            .iter()
            .find(|(_, node)| node.label.as_deref() == Some(LABELS[page]))
            .expect("the cache retains the requested semantic page");
        assert!(node.supports_action(SemanticActionKind::ShowOnScreen));
        self.runtime
            .dispatch_semantic_action(id, SemanticAction::ShowOnScreen)
    }

    fn settle(&mut self) {
        for _ in 0..120 {
            self.frame();
        }
    }
}

#[test]
fn explicit_viewport_cache_mounts_one_extra_page() {
    let controller = PageController::new();
    let host = Host::new(build(
        &controller,
        true,
        Some(ScrollCacheExtent::viewport(1.)),
    ));
    assert_eq!(host.mounted(), [0, 1]);
    let page = host
        .runtime
        .tree()
        .element_with_key(&Key::Value(1))
        .unwrap();
    assert_eq!(
        host.runtime.tree().element_bounds(page).unwrap().origin.x,
        400.
    );
    assert_eq!(controller.offset(), 0.);
}

#[test]
fn cache_defaults_follow_the_implicit_scrolling_switch_and_keep_the_live_page() {
    let controller = PageController::new();
    let mut host = Host::new(build(&controller, false, None));
    assert_eq!(host.mounted(), [0]);
    host.update(build(&controller, true, None));
    assert_eq!(host.mounted(), [0, 1]);
    assert_eq!(controller.page(), Some(0.));
    assert!(controller.jump_to_page(2));
    host.frame();
    assert_eq!(host.mounted(), [1, 2, 3]);
    host.update(build(&controller, false, None));
    assert_eq!(host.mounted(), [2]);
    assert_eq!(controller.page(), Some(2.));
}

#[test]
fn viewport_cache_grows_and_shrinks_without_leaving_a_stale_page() {
    let controller = PageController::new();
    let build_extent = |extent| build(&controller, true, Some(ScrollCacheExtent::viewport(extent)));
    let mut host = Host::new(build_extent(1.));
    assert_eq!(host.mounted(), [0, 1]);
    host.update(build_extent(2.));
    assert_eq!(host.mounted(), [0, 1, 2]);
    host.update(build_extent(1.));
    assert_eq!(host.mounted(), [0, 1]);
    assert_eq!(controller.offset(), 0.);
}

#[test]
fn implicit_cache_adds_the_offscreen_label_to_semantics() {
    let controller = PageController::new();
    let mut host = Host::new(build(&controller, false, None));
    assert_eq!(host.semantic_labels(), ["Page 0"]);
    host.update(build(&controller, true, None));
    assert_eq!(host.semantic_labels(), ["Page 0", "Page 1"]);
}

#[test]
fn explicit_cache_and_implicit_scrolling_must_be_consistent_in_debug() {
    #[cfg(debug_assertions)]
    for (allow, cache) in [
        (true, ScrollCacheExtent::viewport(0.)),
        (false, ScrollCacheExtent::viewport(2.)),
    ] {
        assert!(
            std::panic::catch_unwind(|| {
                let _: Widget = PageView::new([SizedBox::expand()])
                    .allow_implicit_scrolling(allow)
                    .scroll_cache_extent(cache)
                    .into();
            })
            .is_err()
        );
    }
}

#[test]
fn a_full_cached_neighbor_can_reveal_with_explicit_or_default_cache() {
    for cache in [None, Some(ScrollCacheExtent::viewport(1.))] {
        let controller = PageController::new();
        let mut host = Host::new(build(&controller, true, cache));
        assert_eq!(controller.page(), Some(0.));
        assert!(host.reveal(1));
        host.settle();
        assert_eq!((controller.page(), controller.offset()), (Some(1.), 400.));
    }
}

#[test]
fn a_partial_neighbor_moves_immediately_and_then_snaps_back_when_enabled() {
    let controller = PageController::new().with_viewport_fraction(0.625);
    let mut host = Host::new(build(&controller, false, None));
    assert!(!host.reveal(1));
    assert_eq!(controller.offset(), 0.);

    host.update(build(&controller, true, None));
    assert!(host.reveal(1));
    assert_eq!(controller.offset(), 100.);
    assert!(host.runtime.frame_requested());
    host.settle();
    assert_eq!(controller.offset(), 0.);
}

#[test]
fn a_pixel_cache_does_not_scale_with_the_viewport() {
    let pixel = ScrollCacheExtent::pixels(300.);
    let relative = ScrollCacheExtent::viewport(1.);
    for (cache, after_resize) in [(pixel, vec![0, 1, 2]), (relative, vec![0, 1])] {
        let controller = PageController::new();
        let mut host = Host::new(build(&controller, true, Some(cache)));
        assert_eq!(host.mounted(), [0, 1]);
        host.width = 200.;
        host.frame();
        assert_eq!(host.mounted(), after_resize);
        assert_eq!(controller.page(), Some(0.));
    }
}

#[test]
fn cached_semantics_cease_to_be_hidden_when_the_page_is_revealed() {
    let controller = PageController::new();
    let mut host = Host::new(build(&controller, true, None));
    let exposed = |host: &Host, page: usize| {
        host.runtime
            .tree()
            .semantics()
            .iter()
            .find(|(_, node)| node.label.as_deref() == Some(LABELS[page]))
            .map(|(id, node)| (id, !node.state.hidden))
            .expect("cached semantic label")
    };

    let (first, first_visible) = exposed(&host, 0);
    let (second, second_visible) = exposed(&host, 1);
    assert!((first_visible && !second_visible));
    assert!(host.reveal(1));
    host.frame();
    assert_eq!(exposed(&host, 0), (first, false));
    assert_eq!(exposed(&host, 1), (second, true));
}
