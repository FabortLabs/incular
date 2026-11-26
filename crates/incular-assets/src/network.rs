//! Flutter's `NetworkAssetBundle` (`services/asset_bundle.dart`): assets
//! fetched from URLs relative to a base URL.
//!
//! `incular-assets` may not depend on an HTTP client (see
//! `specs/architecture.json`), so the transport is an injected
//! [`AssetFetcher`]. The bundle resolves keys to URLs and turns responses
//! into asset results, as Flutter's bundle does over `HttpClient`.

use std::{fmt, fmt::Write as _, sync::Arc};

use crate::{AssetBundle, AssetError};

/// The response to one GET made by an [`AssetFetcher`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssetResponse {
    /// The HTTP status code.
    pub status: u16,
    /// The response body.
    pub bytes: Arc<[u8]>,
}

impl AssetResponse {
    /// A response with `status` and `bytes`.
    #[must_use]
    pub fn new(status: u16, bytes: impl Into<Arc<[u8]>>) -> Self {
        Self {
            status,
            bytes: bytes.into(),
        }
    }
}

/// Receives the outcome of one [`AssetFetcher::fetch`]: the response, or a
/// transport error message.
pub type AssetFetchCallback = Box<dyn FnOnce(Result<AssetResponse, String>) + Send>;

/// Receives the outcome of one [`AssetBundle::load_async`].
pub type AssetLoadCallback = Box<dyn FnOnce(Result<Arc<[u8]>, AssetError>) + Send>;

/// The transport of a [`NetworkAssetBundle`]: Flutter's `HttpClient`, which
/// the application supplies (for example over `reqwest` natively or the
/// browser's `fetch` on the web).
pub trait AssetFetcher: Send + Sync + 'static {
    /// Starts a GET of `url` and calls `done` exactly once with the
    /// response or a transport error. It must not block: on `wasm32` it runs
    /// on the page thread. `done` may run before `fetch` returns, or later
    /// on any thread; a caller that needs the result on the UI thread
    /// forwards it there.
    fn fetch(&self, url: &str, done: AssetFetchCallback);

    /// A blocking GET of `url` for the synchronous [`AssetBundle::load`],
    /// or `None` when the fetcher cannot block. The default is `None`, and
    /// a fetcher used on `wasm32` must keep it.
    fn fetch_blocking(&self, url: &str) -> Option<Result<AssetResponse, String>> {
        let _ = url;
        None
    }
}

/// Flutter's `NetworkAssetBundle`: resolves each key as a URL relative to a
/// base URL and fetches it through an [`AssetFetcher`]. Nothing is cached,
/// as in Flutter; wrap it in a
/// [`CachingAssetBundle`](crate::CachingAssetBundle) to keep strings.
///
/// Flutter's loads are asynchronous; here [`AssetBundle::load_async`] is
/// that load and never blocks. The synchronous [`AssetBundle::load`] works
/// only with a fetcher that implements
/// [`AssetFetcher::fetch_blocking`], and fails otherwise.
#[derive(Clone)]
pub struct NetworkAssetBundle {
    identity: u64,
    base_url: String,
    fetcher: Arc<dyn AssetFetcher>,
}

impl NetworkAssetBundle {
    /// A bundle whose keys resolve against `base_url`.
    #[must_use]
    pub fn new(base_url: impl Into<String>, fetcher: impl AssetFetcher) -> Self {
        Self::with_shared_fetcher(base_url, Arc::new(fetcher))
    }

    /// A bundle over a fetcher shared with other bundles.
    #[must_use]
    pub fn with_shared_fetcher(
        base_url: impl Into<String>,
        fetcher: Arc<dyn AssetFetcher>,
    ) -> Self {
        static IDS: incular_core::IdGenerator = incular_core::IdGenerator::new();
        Self {
            identity: IDS.next(),
            base_url: base_url.into(),
            fetcher,
        }
    }

    /// The base URL keys resolve against.
    #[must_use]
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Flutter `_urlFromKey`: `baseUrl.resolve(key)`.
    #[must_use]
    pub fn url_for(&self, key: &str) -> String {
        resolve_url(&self.base_url, key)
    }
}

/// Flutter's checks on a response: anything but `200 OK` fails with the
/// status code.
fn asset_from_response(
    key: &str,
    response: Result<AssetResponse, String>,
) -> Result<Arc<[u8]>, AssetError> {
    match response {
        Ok(response) if response.status == 200 => Ok(response.bytes),
        Ok(response) => Err(AssetError::Io {
            key: key.to_owned(),
            message: format!("HTTP status code: {}", response.status),
        }),
        Err(message) => Err(AssetError::Io {
            key: key.to_owned(),
            message,
        }),
    }
}

impl AssetBundle for NetworkAssetBundle {
    fn load(&self, key: &str) -> Result<Arc<[u8]>, AssetError> {
        match self.fetcher.fetch_blocking(&self.url_for(key)) {
            Some(response) => asset_from_response(key, response),
            None => Err(AssetError::Io {
                key: key.to_owned(),
                message: "the fetcher cannot block; use AssetBundle::load_async".to_owned(),
            }),
        }
    }

    fn load_async(&self, key: &str, done: AssetLoadCallback) {
        let owned = key.to_owned();
        self.fetcher.fetch(
            &self.url_for(key),
            Box::new(move |response| done(asset_from_response(&owned, response))),
        );
    }
}

impl fmt::Display for NetworkAssetBundle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "NetworkAssetBundle#{:05x}({})", self.identity & 0x000f_ffff, self.base_url)
    }
}

impl fmt::Debug for NetworkAssetBundle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Flutter `toString`: `NetworkAssetBundle#hash(baseUrl)`.
        f.debug_tuple("NetworkAssetBundle")
            .field(&self.base_url)
            .finish()
    }
}

/// Dart's `Uri.resolve` (RFC 3986 section 5.2) for a base URL and a
/// reference. Characters that may not appear in a URL, such as spaces and
/// non-ASCII text, are percent-encoded in the reference first.
#[must_use]
pub fn resolve_url(base: &str, reference: &str) -> String {
    let reference = encode_reference(reference);
    let (reference_path, reference_tail) = split_tail(&reference);
    if has_scheme(&reference) {
        let (scheme, rest) = reference_path
            .split_once(':')
            .unwrap_or(("", reference_path));
        return match rest.strip_prefix("//") {
            Some(rest) => {
                let (authority, path) = split_authority(rest);
                format!(
                    "{scheme}://{authority}{}{reference_tail}",
                    remove_dot_segments(path)
                )
            }
            None => format!("{scheme}:{}{reference_tail}", remove_dot_segments(rest)),
        };
    }
    let (base_path_and_query, _) = base.split_once('#').unwrap_or((base, ""));
    let (base_without_query, base_query) = base_path_and_query
        .split_once('?')
        .map_or((base_path_and_query, None), |(path, query)| {
            (path, Some(query))
        });
    let (origin, base_path) = split_origin(base_without_query);
    if let Some(network) = reference.strip_prefix("//") {
        let scheme = origin.split_once(':').map_or("", |(scheme, _)| scheme);
        let (path_part, tail) = split_tail(network);
        let (authority, path) = split_authority(path_part);
        return format!("{scheme}://{authority}{}{tail}", remove_dot_segments(path));
    }
    if reference_path.is_empty() {
        // An empty path keeps the base path, and its query unless the
        // reference has one.
        let query = if reference_tail.starts_with('?') {
            String::new()
        } else {
            base_query.map_or_else(String::new, |query| format!("?{query}"))
        };
        return format!("{origin}{base_path}{query}{reference_tail}");
    }
    let merged = if reference_path.starts_with('/') {
        reference_path.to_owned()
    } else if base_path.is_empty() && origin.contains("://") {
        format!("/{reference_path}")
    } else {
        let directory = base_path.rfind('/').map_or("", |end| &base_path[..=end]);
        format!("{directory}{reference_path}")
    };
    format!("{origin}{}{reference_tail}", remove_dot_segments(&merged))
}

/// Whether a URL reference starts with a scheme (`ALPHA *( ALPHA / DIGIT /
/// "+" / "-" / "." ) ":"`).
fn has_scheme(reference: &str) -> bool {
    let Some((scheme, _)) = reference.split_once(':') else {
        return false;
    };
    let mut chars = scheme.chars();
    chars
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic())
        && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'))
        && !scheme.contains('/')
}

/// Splits a reference into its path and its query and fragment.
fn split_tail(reference: &str) -> (&str, &str) {
    let end = reference.find(['?', '#']).unwrap_or(reference.len());
    reference.split_at(end)
}

/// Splits `authority/path` after `//`.
fn split_authority(rest: &str) -> (&str, &str) {
    let end = rest.find('/').unwrap_or(rest.len());
    rest.split_at(end)
}

/// Splits a base URL without query into `scheme://authority` (or
/// `scheme:`) and its path.
fn split_origin(base: &str) -> (&str, &str) {
    if let Some(index) = base.find("://") {
        let after = index + 3;
        let end = base[after..]
            .find('/')
            .map_or(base.len(), |end| after + end);
        return base.split_at(end);
    }
    match base.find(':') {
        Some(index) if has_scheme(base) => base.split_at(index + 1),
        _ => ("", base),
    }
}

/// RFC 3986 `remove_dot_segments`.
fn remove_dot_segments(path: &str) -> String {
    let mut output: Vec<&str> = Vec::new();
    let absolute = path.starts_with('/');
    let segments: Vec<&str> = path.split('/').collect();
    let last = segments.len().saturating_sub(1);
    let mut trailing_slash = false;
    for (index, segment) in segments.iter().enumerate() {
        if index == 0 && absolute {
            continue;
        }
        match *segment {
            "." => trailing_slash = index == last,
            ".." => {
                output.pop();
                trailing_slash = index == last;
            }
            segment => {
                output.push(segment);
                trailing_slash = false;
            }
        }
    }
    let mut result = String::with_capacity(path.len());
    if absolute {
        result.push('/');
    }
    result.push_str(&output.join("/"));
    if trailing_slash && !result.ends_with('/') {
        result.push('/');
    }
    result
}

/// Percent-encodes the characters Dart's `Uri.parse` would encode in a
/// reference: controls, spaces, non-ASCII bytes and `"<>\^`{|}`. Existing
/// escapes are kept.
fn encode_reference(reference: &str) -> String {
    let mut encoded = String::with_capacity(reference.len());
    for byte in reference.bytes() {
        let keep = byte.is_ascii_graphic()
            && !matches!(
                byte,
                b'"' | b'<' | b'>' | b'\\' | b'^' | b'`' | b'{' | b'|' | b'}'
            );
        if keep {
            encoded.push(char::from(byte));
        } else {
            let _ = write!(encoded, "%{byte:02X}");
        }
    }
    encoded
}
