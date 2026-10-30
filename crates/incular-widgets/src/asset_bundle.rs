//! Flutter's `DefaultAssetBundle` (`widgets/basic.dart`).

use std::sync::{Arc, OnceLock};

use incular_assets::{AssetBundle, MemoryAssetBundle};

use crate::{BuildContext, Widget};

/// The bundle a [`DefaultAssetBundle`] provides.
#[derive(Clone)]
struct BundleScope(Arc<dyn AssetBundle>);

/// Flutter's `DefaultAssetBundle`: the [`AssetBundle`] descendants load
/// their assets from.
pub struct DefaultAssetBundle {
    bundle: Arc<dyn AssetBundle>,
    child: Widget,
}

impl DefaultAssetBundle {
    #[must_use]
    pub fn new(bundle: Arc<dyn AssetBundle>, child: impl Into<Widget>) -> Self {
        Self {
            bundle,
            child: child.into(),
        }
    }

    /// `DefaultAssetBundle.of`: the nearest bundle, else the root bundle.
    /// Incular has no packaged application assets, so the root bundle is
    /// empty; an application installs its own bundle near the root.
    #[must_use]
    pub fn of(context: &BuildContext<'_>) -> Arc<dyn AssetBundle> {
        context
            .depend_on::<BundleScope>()
            .map_or_else(root_bundle, |scope| scope.0)
    }
}

/// Flutter's `rootBundle`: the bundle without a `DefaultAssetBundle`.
fn root_bundle() -> Arc<dyn AssetBundle> {
    static ROOT: OnceLock<Arc<dyn AssetBundle>> = OnceLock::new();
    ROOT.get_or_init(|| Arc::new(MemoryAssetBundle::new()))
        .clone()
}

impl From<DefaultAssetBundle> for Widget {
    fn from(value: DefaultAssetBundle) -> Self {
        Widget::environment_scope(BundleScope(value.bundle), value.child)
    }
}
