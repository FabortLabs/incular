//! Flutter's `DefaultAssetBundle` (`widgets/basic.dart`).

use std::sync::Arc;

use incular_assets::{AssetBundle, root_asset_bundle};

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
    /// An unconfigured root is empty. Hosts can configure a root through
    /// [incular_assets::replace_root_asset_bundle].
    #[must_use]
    pub fn of(context: &BuildContext<'_>) -> Arc<dyn AssetBundle> {
        context
            .depend_on::<BundleScope>()
            .map_or_else(root_asset_bundle, |scope| scope.0)
    }
}

impl From<DefaultAssetBundle> for Widget {
    fn from(value: DefaultAssetBundle) -> Self {
        Widget::environment_scope(BundleScope(value.bundle), value.child)
    }
}
