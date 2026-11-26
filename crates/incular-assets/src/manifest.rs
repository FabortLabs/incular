//! Flutter's packaged asset manifest and resolution-variant metadata.

use std::{
    borrow::Cow,
    collections::{HashMap, hash_map::Entry},
    sync::{Arc, Mutex, PoisonError},
};

use base64::{Engine as _, engine::general_purpose::STANDARD};
use incular_core::standard_message_codec::{MessageValue, StandardMessageCodec};

use crate::{AssetBundle, AssetError};

/// The platform-specific envelope around the Flutter asset manifest.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AssetManifestFormat {
    /// The native `AssetManifest.bin`, encoded with the standard message codec.
    Binary,
    /// The web `AssetManifest.bin.json`, a JSON string wrapping base64 bytes.
    WebJson,
}

impl AssetManifestFormat {
    #[must_use]
    pub const fn current_platform() -> Self {
        if cfg!(target_arch = "wasm32") {
            Self::WebJson
        } else {
            Self::Binary
        }
    }

    #[must_use]
    pub const fn asset_key(self) -> &'static str {
        match self {
            Self::Binary => "AssetManifest.bin",
            Self::WebJson => "AssetManifest.bin.json",
        }
    }
}

/// The physical key and optional resolution of an asset variant.
#[derive(Clone, Debug, PartialEq)]
pub struct AssetMetadata {
    /// The physical key to load; it can include a content hash.
    pub key: String,
    /// The declared pixel ratio, or `None` for an unqualified asset.
    pub target_device_pixel_ratio: Option<f64>,
    /// Whether this is the main asset rather than one of its variants.
    pub main: bool,
}

/// The result callback for a manifest load. It can run synchronously or
/// later on the bundle's loading thread.
pub type AssetManifestLoadCallback = Box<dyn FnOnce(Result<AssetManifest, AssetError>) + Send>;

/// A shareable view of the actual packaged assets. Variant lists are
/// validated and materialized only when their logical key is queried.
#[derive(Clone, Debug)]
pub struct AssetManifest {
    format: AssetManifestFormat,
    entries: Arc<Mutex<ManifestEntries>>,
}

#[derive(Debug)]
struct ManifestEntries {
    positions: HashMap<Arc<str>, usize>,
    entries: Vec<(Arc<str>, Variants)>,
    materialized: Vec<usize>,
}

#[derive(Debug)]
enum Variants {
    Raw(MessageValue),
    Parsed(Arc<[AssetMetadata]>),
}

impl AssetManifest {
    /// Loads the manifest using the platform's packaged format. The bundle
    /// controls whether the load is immediate, backed by a browser fetch, or
    /// executed on a loading thread.
    pub fn load_from_asset_bundle(bundle: Arc<dyn AssetBundle>, done: AssetManifestLoadCallback) {
        Self::load_from_asset_bundle_with_format(
            bundle,
            AssetManifestFormat::current_platform(),
            done,
        );
    }

    /// Loads a specific format, including a web manifest on a native host.
    pub fn load_from_asset_bundle_with_format(
        bundle: Arc<dyn AssetBundle>,
        format: AssetManifestFormat,
        done: AssetManifestLoadCallback,
    ) {
        bundle.load_asset_manifest(format, done);
    }

    /// Decodes a native or web manifest. The format requires a map with
    /// string keys. Individual variant records are checked on lookup.
    ///
    /// # Errors
    ///
    /// Returns an asset error for invalid UTF-8, an invalid envelope, or an
    /// invalid top-level map.
    pub fn from_bytes(bytes: &[u8], format: AssetManifestFormat) -> Result<Self, AssetError> {
        let decoded;
        let bytes = if format == AssetManifestFormat::WebJson {
            let text = std::str::from_utf8(bytes)
                .map_err(|_| AssetError::InvalidUtf8(format.asset_key().to_owned()))?;
            let text = text.strip_prefix('\u{feff}').unwrap_or(text);
            // Flutter emits an unescaped base64 string, which need not be
            // copied. The fallback also handles valid escaped JSON strings.
            let encoded = match serde_json::from_str::<&str>(text) {
                Ok(encoded) => Cow::Borrowed(encoded),
                Err(_) => Cow::Owned(
                    serde_json::from_str::<String>(text)
                        .map_err(|error| manifest_error(format, error.to_string()))?,
                ),
            };
            decoded = decode_base64(&encoded)
                .map_err(|error| manifest_error(format, error.to_string()))?;
            &decoded
        } else {
            bytes
        };

        let MessageValue::Map(map) = StandardMessageCodec::decode_message(Some(bytes))
            .map_err(|error| manifest_error(format, error.to_string()))?
        else {
            return Err(manifest_error(
                format,
                "expected a map of logical asset keys",
            ));
        };
        let mut entries: Vec<(Arc<str>, Variants)> = Vec::with_capacity(map.len());
        let mut positions: HashMap<Arc<str>, usize> = HashMap::with_capacity(map.len());
        for (key, variants) in map {
            let MessageValue::String(key) = key else {
                return Err(manifest_error(format, "an asset key is not a string"));
            };
            let key: Arc<str> = Arc::from(key);
            match positions.entry(key.clone()) {
                Entry::Occupied(entry) => {
                    entries[*entry.get()].1 = Variants::Raw(variants);
                }
                Entry::Vacant(entry) => {
                    entry.insert(entries.len());
                    entries.push((key, Variants::Raw(variants)));
                }
            }
        }
        Ok(Self {
            format,
            entries: Arc::new(Mutex::new(ManifestEntries {
                positions,
                entries,
                materialized: Vec::new(),
            })),
        })
    }

    /// Lists logical main asset keys, not the physical keys of their
    /// variants. As in Flutter, still-unqueried keys precede keys already
    /// materialized, which are listed in their first query order.
    #[must_use]
    pub fn list_assets(&self) -> Vec<Arc<str>> {
        let state = self.entries.lock().unwrap_or_else(PoisonError::into_inner);
        state
            .entries
            .iter()
            .filter(|(_, variants)| matches!(variants, Variants::Raw(_)))
            .map(|(key, _)| key.clone())
            .chain(
                state
                    .materialized
                    .iter()
                    .map(|&index| state.entries[index].0.clone()),
            )
            .collect()
    }

    /// Looks up an exact logical key, including the main file when present.
    /// An unknown key is `None`; a known key can have an empty variant list.
    /// Successful lookups reuse their typed list across manifest handles.
    ///
    /// # Errors
    ///
    /// Returns a parse error if this key's variant list has invalid metadata.
    pub fn get_asset_variants(
        &self,
        logical: &str,
    ) -> Result<Option<Arc<[AssetMetadata]>>, AssetError> {
        let mut state = self.entries.lock().unwrap_or_else(PoisonError::into_inner);
        let Some(&index) = state.positions.get(logical) else {
            return Ok(None);
        };
        let variants = &mut state.entries[index].1;
        let rows = match variants {
            Variants::Parsed(parsed) => return Ok(Some(parsed.clone())),
            Variants::Raw(MessageValue::Null) => return Ok(None),
            Variants::Raw(MessageValue::List(rows)) => rows,
            Variants::Raw(_) => return Err(self.variant_error(logical, "expected a variant list")),
        };
        // Check the full list before taking ownership of any fields so a
        // malformed sibling can't leave this row only partly converted.
        let mut fields = Vec::with_capacity(rows.len());
        for row in rows {
            let MessageValue::Map(fields_in_row) = row else {
                return Err(self.variant_error(logical, "a variant is not a map"));
            };
            let field = |name| {
                fields_in_row
                    .iter()
                    .rposition(|(key, _)| matches!(key, MessageValue::String(key) if key == name))
            };
            let asset =
                field("asset").filter(|&i| matches!(fields_in_row[i].1, MessageValue::String(_)));
            let Some(asset) = asset else {
                return Err(self.variant_error(logical, "a variant has no string asset key"));
            };
            let dpr = match field("dpr").map(|i| &fields_in_row[i].1) {
                Some(MessageValue::Float(value)) => Some(*value),
                None | Some(MessageValue::Null) => None,
                _ => return Err(self.variant_error(logical, "a variant's dpr is not a double")),
            };
            fields.push((asset, dpr));
        }
        let Variants::Raw(MessageValue::List(rows)) =
            std::mem::replace(variants, Variants::Raw(MessageValue::Null))
        else {
            unreachable!("variant list was validated above");
        };
        let parsed: Arc<[_]> = rows
            .into_iter()
            .zip(fields)
            .map(|(row, (asset, dpr))| {
                let MessageValue::Map(mut row) = row else {
                    unreachable!("validated variant map")
                };
                let MessageValue::String(key) = row.swap_remove(asset).1 else {
                    unreachable!("validated asset key")
                };
                AssetMetadata {
                    main: key == logical || matches_without_hash(&key, logical),
                    key,
                    target_device_pixel_ratio: dpr,
                }
            })
            .collect();
        *variants = Variants::Parsed(parsed.clone());
        state.materialized.push(index);
        Ok(Some(parsed))
    }

    fn variant_error(&self, logical: &str, message: &str) -> AssetError {
        manifest_error(self.format, format!("{logical:?}: {message}"))
    }
}

fn manifest_error(format: AssetManifestFormat, message: impl Into<String>) -> AssetError {
    AssetError::Parse {
        key: format.asset_key().to_owned(),
        message: message.into(),
    }
}

fn decode_base64(encoded: &str) -> Result<Vec<u8>, base64::DecodeError> {
    match STANDARD.decode(encoded) {
        Ok(bytes) => Ok(bytes),
        Err(error) => {
            // Dart accepts both alphabets in the same input plus escaped
            // equals padding, but it does not accept missing padding.
            if !encoded
                .bytes()
                .any(|byte| matches!(byte, b'-' | b'_' | b'%'))
            {
                return Err(error);
            }
            let normalized = encoded
                .replace("%3D", "=")
                .replace("%3d", "=")
                .replace('-', "+")
                .replace('_', "/");
            STANDARD.decode(normalized)
        }
    }
}

fn matches_without_hash(path: &str, logical: &str) -> bool {
    // Match the leftmost suffix permitted by Flutter's regexp: an eight
    // digit lowercase hex hash and no extension, one extension, or
    // .js.map/.wasm.map/.mjs.map. A hash in a parent folder is never removed.
    let start = path.rfind('/').map_or(0, |index| index + 1);
    let mut candidate = None;
    for (index, _) in path[start..].rmatch_indices('.').take(3) {
        let dot = start + index;
        let tail = &path[dot + 1..];
        let Some(hash) = tail.as_bytes().get(..8) else {
            continue;
        };
        if !hash
            .iter()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
        {
            continue;
        }
        let extension = &tail[8..];
        if extension.is_empty()
            || matches!(extension, ".js.map" | ".wasm.map" | ".mjs.map")
            || extension
                .strip_prefix('.')
                .is_some_and(|ext| !ext.is_empty() && !ext.contains('.'))
        {
            candidate = Some((dot, extension));
        }
    }
    candidate.is_some_and(|(dot, extension)| logical.strip_prefix(&path[..dot]) == Some(extension))
}
