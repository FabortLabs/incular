//! Consolidate a streamed response into bytes without tying it to an HTTP client.

use std::{convert::Infallible, fmt, future::poll_fn, io::Write, pin::pin};

use flate2::write::MultiGzDecoder;
use futures_core::Stream;

/// Whether the client has already uncompressed the response body.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum HttpResponseCompressionState {
    #[default]
    NotCompressed,
    Compressed,
    Decompressed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HttpResponseOptions {
    /// Declared wire length, or none when the server did not supply one.
    pub content_length: Option<u64>,
    pub compression: HttpResponseCompressionState,
    /// Uncompress a still-gzipped response. Already decoded bytes are
    /// never fed through the decoder again.
    pub auto_uncompress: bool,
}

impl Default for HttpResponseOptions {
    fn default() -> Self {
        Self {
            content_length: None,
            compression: HttpResponseCompressionState::NotCompressed,
            auto_uncompress: true,
        }
    }
}

#[derive(Debug)]
pub enum ConsolidateResponseError<E, P = Infallible> {
    Response(E),
    Progress(P),
    Decompression(std::io::Error),
    ByteCountOverflow,
}

impl<E: fmt::Display, P: fmt::Display> fmt::Display for ConsolidateResponseError<E, P> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Response(error) => write!(f, "while reading the HTTP response: {error}"),
            Self::Progress(error) => write!(f, "from the HTTP progress callback: {error}"),
            Self::Decompression(error) => write!(f, "while decoding the gzip response: {error}"),
            Self::ByteCountOverflow => f.write_str("the HTTP response byte counter overflowed"),
        }
    }
}

impl<E, P> std::error::Error for ConsolidateResponseError<E, P>
where
    E: std::error::Error + 'static,
    P: std::error::Error + 'static,
{
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Response(error) => Some(error),
            Self::Progress(error) => Some(error),
            Self::Decompression(error) => Some(error),
            Self::ByteCountOverflow => None,
        }
    }
}

/// Reads the whole streamed body, with automatic gzip decoding by default.
pub async fn consolidate_http_client_response_bytes<S, B, E>(
    body: S,
    options: HttpResponseOptions,
) -> Result<Vec<u8>, ConsolidateResponseError<E>>
where
    S: Stream<Item = Result<B, E>>,
    B: AsRef<[u8]>,
{
    consolidate_http_client_response_bytes_with_progress(body, options, |_, _| Ok(())).await
}

/// Reports cumulative bytes received for every input chunk, before any
/// auto-decompression changes the output size. When the HTTP client has
/// already decompressed the body, the original declared length is not useful.
/// A body error or callback error returns immediately and drops the stream.
pub async fn consolidate_http_client_response_bytes_with_progress<S, B, E, P>(
    body: S,
    options: HttpResponseOptions,
    mut on_bytes_received: impl FnMut(u64, Option<u64>) -> Result<(), P>,
) -> Result<Vec<u8>, ConsolidateResponseError<E, P>>
where
    S: Stream<Item = Result<B, E>>,
    B: AsRef<[u8]>,
{
    let expected = if options.compression == HttpResponseCompressionState::Decompressed {
        None
    } else {
        options.content_length
    };
    let mut output = if options.auto_uncompress
        && options.compression == HttpResponseCompressionState::Compressed
    {
        Output::Gzip(MultiGzDecoder::new(Vec::new()))
    } else {
        // A response header is only an estimate. Bound speculative
        // reservation and grow with actual bytes even when it understates.
        Output::Uncompressed(Vec::with_capacity(
            expected.unwrap_or(0).min(64 * 1024) as usize
        ))
    };
    let mut body = pin!(body);
    let mut count = 0_u64;
    while let Some(chunk) = poll_fn(|context| body.as_mut().poll_next(context)).await {
        let chunk = chunk.map_err(ConsolidateResponseError::Response)?;
        let bytes = chunk.as_ref();
        output
            .write(bytes)
            .map_err(ConsolidateResponseError::Decompression)?;
        count = count
            .checked_add(bytes.len() as u64)
            .ok_or(ConsolidateResponseError::ByteCountOverflow)?;
        on_bytes_received(count, expected).map_err(ConsolidateResponseError::Progress)?;
    }
    output
        .finish()
        .map_err(ConsolidateResponseError::Decompression)
}

enum Output {
    Uncompressed(Vec<u8>),
    Gzip(MultiGzDecoder<Vec<u8>>),
}

impl Output {
    fn write(&mut self, chunk: &[u8]) -> std::io::Result<()> {
        match self {
            Self::Uncompressed(bytes) => {
                bytes.extend_from_slice(chunk);
                Ok(())
            }
            Self::Gzip(decoder) => decoder.write_all(chunk),
        }
    }

    fn finish(self) -> std::io::Result<Vec<u8>> {
        match self {
            Self::Uncompressed(bytes) => Ok(bytes),
            Self::Gzip(decoder) => decoder.finish(),
        }
    }
}
