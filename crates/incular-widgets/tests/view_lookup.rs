//! Flutter's `View.of` and `View.maybeOf`.

use std::{cell::RefCell, rc::Rc};

use incular_config::Constraints;
use incular_core::Size;
use incular_widgets::internal::WidgetTree;
use incular_widgets::{LayoutBuilder, View, ViewController, ViewId, ViewMetrics, Widget};

fn mount(widget: Widget) -> WidgetTree {
    let mut tree = WidgetTree::new();
    tree.mount(widget).expect("mount");
    tree.layout(Constraints::tight(Size::new(100., 100.)))
        .expect("layout");
    tree
}

#[test]
fn a_descendant_finds_the_nearest_view() {
    let seen = Rc::new(RefCell::new(Vec::new()));
    let probe = {
        let seen = seen.clone();
        Widget::from(LayoutBuilder::new(move |context, _| {
            seen.borrow_mut()
                .push(View::maybe_of(context).map(|view| view.id));
            Widget::box_(Size::new(1., 1.), incular_core::Color::WHITE)
        }))
    };
    let inner = View::new(ViewId::new(7), probe.clone());
    let _tree = mount(View::new(ViewId::new(3), inner).into());
    let _bare = mount(probe);
    assert_eq!(*seen.borrow(), [Some(ViewId::new(7)), None]);
}

#[test]
fn a_view_change_reaches_its_dependents() {
    let controller = ViewController::new(
        ViewId::new(1),
        ViewMetrics::new(100, 100, 1.),
        Default::default(),
    );
    let sizes = Rc::new(RefCell::new(Vec::new()));
    let probe = {
        let sizes = sizes.clone();
        Widget::from(LayoutBuilder::new(move |context, _| {
            sizes
                .borrow_mut()
                .push(View::of(context).metrics.logical_size());
            Widget::box_(Size::new(1., 1.), incular_core::Color::WHITE)
        }))
    };
    let mut tree = mount(View::with_controller(controller.clone(), probe).into());
    controller.update_metrics(ViewMetrics::new(200, 100, 2.));
    tree.layout(Constraints::tight(Size::new(100., 100.)))
        .expect("layout");
    assert_eq!(
        *sizes.borrow(),
        [Size::new(100., 100.), Size::new(100., 50.)]
    );
}

#[test]
#[should_panic(expected = "no View ancestor")]
fn view_of_without_a_view_panics_as_flutter_throws() {
    let _ = mount(Widget::from(LayoutBuilder::new(|context, _| {
        let _ = View::of(context);
        Widget::box_(Size::new(1., 1.), incular_core::Color::WHITE)
    })));
}
