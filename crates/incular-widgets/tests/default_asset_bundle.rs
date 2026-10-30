//! Flutter's `DefaultAssetBundle`.

use std::{cell::RefCell, rc::Rc, sync::Arc};

use incular_assets::{AssetBundle, MemoryAssetBundle};
use incular_config::Constraints;
use incular_core::{Color, Size};
use incular_widgets::internal::WidgetTree;
use incular_widgets::{Builder, DefaultAssetBundle, Widget};

fn read(widget: impl FnOnce(Widget) -> Widget) -> Option<String> {
    let seen = Rc::new(RefCell::new(None));
    let reader = {
        let seen = seen.clone();
        Builder::new(move |context| {
            *seen.borrow_mut() = Some(
                DefaultAssetBundle::of(context)
                    .load_string("greeting", true)
                    .ok()
                    .map(|text| text.to_string()),
            );
            Widget::box_(Size::ZERO, Color::WHITE)
        })
    };
    let mut tree = WidgetTree::new();
    tree.mount(widget(reader.into())).expect("mount");
    tree.layout(Constraints::loose(Size::new(10., 10.)))
        .expect("layout");
    seen.borrow_mut().take().expect("built")
}

#[test]
fn descendants_load_from_the_nearest_bundle() {
    let bundle = |text: &str| -> Arc<dyn AssetBundle> {
        Arc::new(MemoryAssetBundle::new().with("greeting", text.as_bytes().to_vec()))
    };
    // The root bundle is empty.
    assert_eq!(read(|child| child), None);
    assert_eq!(
        read(|child| DefaultAssetBundle::new(bundle("hello"), child).into()),
        Some("hello".into())
    );
    assert_eq!(
        read(|child| {
            DefaultAssetBundle::new(
                bundle("outer"),
                DefaultAssetBundle::new(bundle("inner"), child),
            )
            .into()
        }),
        Some("inner".into())
    );
}
