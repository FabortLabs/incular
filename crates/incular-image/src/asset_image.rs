//! Bundle identity and Flutter's resolution-aware and exact image sources.

use std::{
    borrow::Cow,
    cmp::Ordering,
    fmt,
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
    sync::Arc,
};

use incular_assets::{
    AssetBundle, AssetManifest, AssetManifestFormat, AssetMetadata, configured_root_asset_bundle,
    root_asset_bundle,
};

use crate::{
    ImageConfiguration, ImageError, ImageHandle, ImageLoadCallback, ImageProvider, ImageSource,
    ImageStream, finite_positive_or_one,
};

/// A shared bundle with identity-based equality. Two distinct bundles can
/// each own an asset of the same name and density without aliasing.
#[derive(Clone)]
pub struct ImageAssetBundle(Arc<dyn AssetBundle>);

impl ImageAssetBundle {
    #[must_use]
    pub fn new(bundle: Arc<dyn AssetBundle>) -> Self {
        Self(bundle)
    }

    #[must_use]
    pub fn shared(&self) -> &Arc<dyn AssetBundle> {
        &self.0
    }
}

impl fmt::Debug for ImageAssetBundle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "AssetBundle({:p})", Arc::as_ptr(&self.0))
    }
}

impl PartialEq for ImageAssetBundle {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for ImageAssetBundle {}

impl Hash for ImageAssetBundle {
    fn hash<H: Hasher>(&self, state: &mut H) {
        (Arc::as_ptr(&self.0) as *const ()).hash(state);
    }
}

/// Flutter's `AssetBundleImageKey`: the physical asset, its scale and its
/// owning bundle. It does not contain the requesting device's ratio.
#[derive(Clone, Debug)]
pub struct AssetBundleImageKey {
    pub bundle: ImageAssetBundle,
    pub name: String,
    pub scale: f64,
}

impl AssetBundleImageKey {
    #[must_use]
    pub fn new(bundle: ImageAssetBundle, name: impl Into<String>, scale: f64) -> Self {
        Self {
            bundle,
            name: name.into(),
            scale,
        }
    }

    /// Decodes the selected physical image from an immediate bundle.
    ///
    /// # Errors
    ///
    /// Returns the bundle or decode error without retrying a different
    /// format or resolution variant.
    pub fn load(&self) -> Result<ImageHandle, ImageError> {
        self.decode(&self.bundle.0.load(&self.name)?)
    }

    /// Loads the selected asset over the bundle's callback transport.
    pub fn load_async(self, done: ImageLoadCallback) {
        let Self {
            bundle,
            name,
            scale,
        } = self;
        let requested = name.clone();
        bundle.0.load_async(
            &requested,
            Box::new(move |outcome| {
                done(
                    outcome
                        .map_err(ImageError::from)
                        .and_then(|bytes| decode_asset(&bytes, name, scale)),
                );
            }),
        );
    }

    fn decode(&self, bytes: &[u8]) -> Result<ImageHandle, ImageError> {
        decode_asset(bytes, self.name.clone(), self.scale)
    }
}

fn decode_asset(bytes: &[u8], name: String, scale: f64) -> Result<ImageHandle, ImageError> {
    let mut handle = ImageHandle::decode(bytes, ImageSource::Asset(name))?;
    handle.set_decoded_scale(scale);
    Ok(handle)
}

fn comparable_scale(scale: f64) -> u64 {
    if scale == 0.0 {
        0
    } else if scale.is_nan() {
        f64::NAN.to_bits()
    } else {
        scale.to_bits()
    }
}

impl PartialEq for AssetBundleImageKey {
    fn eq(&self, other: &Self) -> bool {
        self.bundle == other.bundle
            && self.name == other.name
            && comparable_scale(self.scale) == comparable_scale(other.scale)
    }
}

impl Eq for AssetBundleImageKey {}

impl Hash for AssetBundleImageKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.bundle.hash(state);
        self.name.hash(state);
        comparable_scale(self.scale).hash(state);
    }
}

/// Completion of asynchronous image-key lookup.
pub type AssetImageKeyCallback = Box<dyn FnOnce(Result<AssetBundleImageKey, ImageError>) + Send>;

/// A Flutter resolution-aware asset. An explicitly specified bundle wins,
/// then the configuration bundle, then the root. As an Incular extension,
/// unbundled filesystem paths still use sibling density directories.
#[derive(Clone, Debug)]
pub struct AssetImage {
    path: PathBuf,
    package: Option<String>,
    bundle: Option<ImageAssetBundle>,
}

impl AssetImage {
    #[must_use]
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            package: None,
            bundle: None,
        }
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    #[must_use]
    pub fn bundle(mut self, bundle: Arc<dyn AssetBundle>) -> Self {
        self.bundle = Some(ImageAssetBundle::new(bundle));
        self
    }

    #[must_use]
    pub fn package(mut self, package: impl Into<String>) -> Self {
        self.package = Some(package.into());
        self
    }

    /// The exact logical key in the bundle, before manifest resolution.
    /// Non-UTF-8 filenames have no bundled key but can be loaded by the
    /// unbundled filesystem adapter.
    #[must_use]
    pub fn key_name(&self) -> Option<Cow<'_, str>> {
        self.path
            .to_str()
            .map(|path| key_name(path, self.package.as_deref()))
    }

    /// Loads the bundle manifest and resolves the key, without decoding
    /// image pixels. A warm manifest can complete before this call returns.
    pub fn obtain_key(&self, config: &ImageConfiguration, done: AssetImageKeyCallback) {
        self.obtain_key_from_bundle(select_bundle(self.bundle.as_ref(), config), config, done);
    }

    fn obtain_key_from_bundle(
        &self,
        bundle: ImageAssetBundle,
        config: &ImageConfiguration,
        done: AssetImageKeyCallback,
    ) {
        let Some(logical) = self.key_name().map(Cow::into_owned) else {
            done(Err(non_utf8_asset()));
            return;
        };
        let target = config.target_device_pixel_ratio();
        AssetManifest::load_from_asset_bundle(
            bundle.0.clone(),
            Box::new(move |result| {
                done(
                    result
                        .map_err(ImageError::from)
                        .and_then(|manifest| resolve_key(bundle, logical, target, &manifest)),
                );
            }),
        );
    }

    fn available_bundle(&self, config: &ImageConfiguration) -> Option<ImageAssetBundle> {
        let bundle = self.bundle.clone().or_else(|| {
            if self.path.is_absolute() && self.package.is_none() {
                return None;
            }
            config
                .bundle
                .clone()
                .or_else(|| configured_root_asset_bundle().map(ImageAssetBundle::new))
        });
        bundle.or_else(|| self.package.as_ref().map(|_| select_bundle(None, config)))
    }

    fn bundled_load(
        &self,
        config: &ImageConfiguration,
        bundle: ImageAssetBundle,
    ) -> Result<ImageHandle, ImageError> {
        let logical = self.key_name().ok_or_else(non_utf8_asset)?.into_owned();
        let manifest = bundle
            .0
            .read_asset_manifest(AssetManifestFormat::current_platform())?;
        resolve_key(
            bundle,
            logical,
            config.target_device_pixel_ratio(),
            &manifest,
        )?
        .load()
    }

    /// The path chosen by the explicit filesystem-only adapter.
    #[must_use]
    pub fn variant_path(&self, device_pixel_ratio: f64) -> PathBuf {
        self.file_variant(device_pixel_ratio).0
    }

    fn file_variant(&self, device_pixel_ratio: f64) -> (PathBuf, f64) {
        let wanted = finite_positive_or_one(device_pixel_ratio);
        let main = self.path.is_file().then(|| (1.0, self.path.clone()));
        best_variant(wanted, main.into_iter().chain(self.file_variants()))
            .map_or_else(|| (self.path.clone(), 1.0), |(scale, path)| (path, scale))
    }

    fn file_variants(&self) -> impl Iterator<Item = (f64, PathBuf)> + '_ {
        let name = self.path.file_name();
        let directory = self
            .path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        // Keep absolute/path-like sources as a native extension. Packaged
        // assets never hit the filesystem or infer density from a label.
        name.into_iter().flat_map(move |name| {
            std::fs::read_dir(directory)
                .into_iter()
                .flatten()
                .filter_map(move |entry| {
                    let entry = entry.ok()?;
                    if !entry.file_type().is_ok_and(|t| t.is_dir()) {
                        return None;
                    }
                    let dir_name = entry.file_name();
                    let scale = dir_name.to_str()?.strip_suffix('x')?.parse::<f64>().ok()?;
                    let path = entry.path().join(name);
                    (scale.is_finite() && scale > 0. && path.is_file()).then_some((scale, path))
                })
        })
    }
}

impl PartialEq for AssetImage {
    fn eq(&self, other: &Self) -> bool {
        self.bundle == other.bundle
            && match (self.key_name(), other.key_name()) {
                (Some(a), Some(b)) => a == b,
                (None, None) => self.path == other.path && self.package == other.package,
                _ => false,
            }
    }
}

impl Eq for AssetImage {}

impl ImageProvider for AssetImage {
    fn load(&self) -> Result<ImageHandle, ImageError> {
        self.load_with(&ImageConfiguration::empty())
    }

    fn load_with(&self, config: &ImageConfiguration) -> Result<ImageHandle, ImageError> {
        if let Some(bundle) = self.available_bundle(config) {
            return self.bundled_load(config, bundle);
        }
        let (path, scale) = config.target_device_pixel_ratio().map_or_else(
            || (self.path.clone(), 1.0),
            |ratio| self.file_variant(ratio),
        );
        let mut handle = ImageHandle::from_file(path)?;
        handle.set_decoded_scale(scale);
        Ok(handle)
    }

    fn resolve(&self, config: &ImageConfiguration) -> ImageStream {
        let Some(bundle) = self.available_bundle(config) else {
            return ImageStream::from_result(self.load_with(config));
        };
        ImageStream::from_load_callback(|done| {
            self.obtain_key_from_bundle(
                bundle,
                config,
                Box::new(move |key| match key {
                    Ok(key) => key.load_async(done),
                    Err(error) => done(Err(error)),
                }),
            );
        })
    }
}

/// A bundled image loaded at an explicitly chosen scale. It never reads
/// the manifest or tries density alternatives.
#[derive(Clone, Debug)]
pub struct ExactAssetImage {
    asset: String,
    package: Option<String>,
    bundle: Option<ImageAssetBundle>,
    scale: f64,
}

impl ExactAssetImage {
    #[must_use]
    pub fn new(asset: impl Into<String>) -> Self {
        Self {
            asset: asset.into(),
            package: None,
            bundle: None,
            scale: 1.0,
        }
    }

    #[must_use]
    pub fn bundle(mut self, bundle: Arc<dyn AssetBundle>) -> Self {
        self.bundle = Some(ImageAssetBundle::new(bundle));
        self
    }

    #[must_use]
    pub fn package(mut self, package: impl Into<String>) -> Self {
        self.package = Some(package.into());
        self
    }

    #[must_use]
    pub fn scale(mut self, scale: f64) -> Self {
        self.scale = scale;
        self
    }

    #[must_use]
    pub fn key_name(&self) -> Cow<'_, str> {
        key_name(&self.asset, self.package.as_deref())
    }

    #[must_use]
    pub fn obtain_key(&self, configuration: &ImageConfiguration) -> AssetBundleImageKey {
        AssetBundleImageKey::new(
            select_bundle(self.bundle.as_ref(), configuration),
            self.key_name(),
            self.scale,
        )
    }
}

impl PartialEq for ExactAssetImage {
    fn eq(&self, other: &Self) -> bool {
        self.key_name() == other.key_name()
            && self.bundle == other.bundle
            && comparable_scale(self.scale) == comparable_scale(other.scale)
    }
}

impl Eq for ExactAssetImage {}

impl ImageProvider for ExactAssetImage {
    fn load(&self) -> Result<ImageHandle, ImageError> {
        self.load_with(&ImageConfiguration::empty())
    }

    fn load_with(&self, configuration: &ImageConfiguration) -> Result<ImageHandle, ImageError> {
        self.obtain_key(configuration).load()
    }

    fn resolve(&self, configuration: &ImageConfiguration) -> ImageStream {
        ImageStream::from_load_callback(|done| self.obtain_key(configuration).load_async(done))
    }
}

fn select_bundle(
    own: Option<&ImageAssetBundle>,
    configuration: &ImageConfiguration,
) -> ImageAssetBundle {
    own.or(configuration.bundle.as_ref())
        .cloned()
        .unwrap_or_else(|| ImageAssetBundle::new(root_asset_bundle()))
}

fn non_utf8_asset() -> ImageError {
    ImageError::Io("bundled asset key is not UTF-8".into())
}

fn key_name<'a>(asset: &'a str, package: Option<&str>) -> Cow<'a, str> {
    package.map_or(Cow::Borrowed(asset), |package| {
        Cow::Owned(format!("packages/{package}/{asset}"))
    })
}

fn resolve_key(
    bundle: ImageAssetBundle,
    logical: String,
    target: Option<f64>,
    manifest: &AssetManifest,
) -> Result<AssetBundleImageKey, ImageError> {
    // Flutter visits the row even when DPR is absent, so malformed
    // metadata still surfaces instead of being silently ignored.
    let variants = manifest.get_asset_variants(&logical)?;
    if let (Some(target), Some(variants)) = (target, variants)
        && let Some((scale, chosen)) = best_variant(
            target,
            variants
                .iter()
                .map(|v: &AssetMetadata| (v.target_device_pixel_ratio.unwrap_or(1.0), v)),
        )
    {
        return Ok(AssetBundleImageKey::new(bundle, &chosen.key, scale));
    }
    Ok(AssetBundleImageKey::new(bundle, logical, 1.0))
}

fn flutter_order(a: f64, b: f64) -> Ordering {
    // Dart's comparison sorts all NaNs after numbers, including infinities.
    match (a.is_nan(), b.is_nan()) {
        (true, true) => Ordering::Equal,
        (true, false) => Ordering::Greater,
        (false, true) => Ordering::Less,
        (false, false) => a.total_cmp(&b),
    }
}

fn best_variant<T>(
    target: f64,
    candidates: impl IntoIterator<Item = (f64, T)>,
) -> Option<(f64, T)> {
    let mut exact = None;
    let mut lower = None;
    let mut upper = None;
    // Flutter materializes a sorted map per lookup. The same result needs
    // only the nearest neighbor on each side, with last duplicate winning.
    for (scale, item) in candidates {
        match flutter_order(scale, target) {
            Ordering::Equal => exact = Some((scale, item)),
            Ordering::Less
                if lower
                    .as_ref()
                    .is_none_or(|(previous, _)| !flutter_order(scale, *previous).is_lt()) =>
            {
                lower = Some((scale, item));
            }
            Ordering::Greater
                if upper
                    .as_ref()
                    .is_none_or(|(previous, _)| !flutter_order(scale, *previous).is_gt()) =>
            {
                upper = Some((scale, item));
            }
            _ => {}
        }
    }
    exact.or_else(|| match (lower, upper) {
        (Some(low), Some(high)) if target < 2.0 || target > (low.0 + high.0) / 2.0 => Some(high),
        (Some(low), _) => Some(low),
        (None, high) => high,
    })
}
