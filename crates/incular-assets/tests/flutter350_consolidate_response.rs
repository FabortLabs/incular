use flate2::{Compression, write::GzEncoder};
use futures_core::Stream;
use incular_assets::{
    ConsolidateResponseError, HttpResponseCompressionState as CompressionState,
    HttpResponseOptions, consolidate_http_client_response_bytes as consolidate,
    consolidate_http_client_response_bytes_with_progress as consolidate_progress,
};
use std::{
    cell::Cell,
    collections::VecDeque,
    future::Future,
    io::Write,
    pin::{Pin, pin},
    rc::Rc,
    task::{Context, Poll, Waker},
};

const FULL: [u8; 11] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10];

struct Chunks {
    values: VecDeque<Result<Vec<u8>, &'static str>>,
    polls: Rc<Cell<usize>>,
    dropped: Rc<Cell<bool>>,
    pause: bool,
}

impl Stream for Chunks {
    type Item = Result<Vec<u8>, &'static str>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.polls.set(self.polls.get() + 1);
        if std::mem::take(&mut self.pause) {
            cx.waker().wake_by_ref();
            return Poll::Pending;
        }
        Poll::Ready(self.values.pop_front())
    }
}

impl Drop for Chunks {
    fn drop(&mut self) {
        self.dropped.set(true);
    }
}

fn body(values: impl IntoIterator<Item = Result<Vec<u8>, &'static str>>) -> Chunks {
    Chunks {
        values: values.into_iter().collect(),
        polls: Rc::new(Cell::new(0)),
        dropped: Rc::new(Cell::new(false)),
        pause: false,
    }
}

fn halves(bytes: &[u8], first_len: usize) -> Chunks {
    body([
        Ok(bytes[..first_len].to_vec()),
        Ok(bytes[first_len..].to_vec()),
    ])
}

fn complete<T>(future: impl Future<Output = T>) -> T {
    match pin!(future).poll(&mut Context::from_waker(Waker::noop())) {
        Poll::Ready(value) => value,
        Poll::Pending => panic!("test chunks are ready"),
    }
}

fn gzip() -> Vec<u8> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&FULL).unwrap();
    encoder.finish().unwrap()
}

#[test]
fn chunks_are_joined_despite_known_short_or_absent_content_length() {
    for content_length in [Some(11), Some(6), None] {
        let bytes = complete(consolidate(
            halves(&FULL, 6),
            HttpResponseOptions {
                content_length,
                ..Default::default()
            },
        ))
        .unwrap();
        assert_eq!(bytes, FULL);
    }
}

#[test]
fn callback_reports_cumulative_bytes_on_each_chunk() {
    let mut events = Vec::new();
    let bytes = complete(consolidate_progress(
        halves(&FULL, 6),
        HttpResponseOptions {
            content_length: Some(22),
            ..Default::default()
        },
        |n, expected| {
            events.push((n, expected));
            Ok::<_, &'static str>(())
        },
    ))
    .unwrap();

    assert_eq!(bytes, FULL);
    assert_eq!(events, [(6, Some(22)), (11, Some(22))]);
}

#[test]
fn read_error_and_progress_error_stop_consumption_and_preserve_the_reason() {
    let input = body([Err("Test Error"), Ok(FULL.to_vec())]);
    let polls = input.polls.clone();
    let dropped = input.dropped.clone();
    assert!(matches!(
        complete(consolidate(input, Default::default())),
        Err(ConsolidateResponseError::Response("Test Error"))
    ));
    assert_eq!(polls.get(), 1);
    assert!(dropped.get());

    let input = halves(&FULL, 6);
    let polls = input.polls.clone();
    let dropped = input.dropped.clone();
    let mut called = Vec::new();
    let result = complete(consolidate_progress(
        input,
        Default::default(),
        |n, expected| {
            called.push((n, expected));
            Err("misbehaving callback")
        },
    ));
    assert!(matches!(
        result,
        Err(ConsolidateResponseError::Progress("misbehaving callback"))
    ));
    assert_eq!(called, [(6, None)]);
    assert_eq!(polls.get(), 1);
    assert!(dropped.get());
}

#[test]
fn gzip_is_uncompressed_by_default_or_preserved_verbatim_when_requested() {
    let compressed = gzip();
    let options = HttpResponseOptions {
        compression: CompressionState::Compressed,
        content_length: Some(compressed.len() as u64),
        ..Default::default()
    };
    let collect = |options| {
        complete(consolidate(
            halves(&compressed, compressed.len() / 2),
            options,
        ))
        .unwrap()
    };
    assert_eq!(collect(options), FULL);
    assert_eq!(
        collect(HttpResponseOptions {
            auto_uncompress: false,
            ..options
        }),
        compressed
    );
}

#[test]
fn progress_keeps_wire_lengths_or_suppresses_the_predecode_total() {
    let bytes = gzip();
    let half = bytes.len() / 2;
    let n = bytes.len() as u64;
    for (compression, declared, expected) in [
        (CompressionState::Compressed, Some(n), Some(n)),
        (CompressionState::Decompressed, Some(22), None),
    ] {
        let mut progress = Vec::new();
        let result = complete(consolidate_progress(
            halves(&bytes, half),
            HttpResponseOptions {
                compression,
                content_length: declared,
                ..Default::default()
            },
            |n, length| {
                progress.push((n, length));
                Ok::<_, &'static str>(())
            },
        ))
        .unwrap();
        assert_eq!(progress, [(half as u64, expected), (n, expected)]);
        let expected_bytes = if compression == CompressionState::Compressed {
            &FULL[..]
        } else {
            &bytes[..]
        };
        assert_eq!(result, expected_bytes);
    }
}

#[test]
fn the_future_waits_when_the_transport_has_not_yet_supplied_a_chunk() {
    let mut input = halves(&FULL, 6);
    input.pause = true;
    let mut future = pin!(consolidate(input, Default::default()));
    let mut cx = Context::from_waker(Waker::noop());
    assert!(future.as_mut().poll(&mut cx).is_pending());
    assert!(matches!(future.as_mut().poll(&mut cx), Poll::Ready(Ok(bytes)) if bytes == FULL));
}
