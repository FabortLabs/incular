use std::{
    cell::{Cell, RefCell},
    convert::Infallible,
    future::{Future, poll_fn, ready},
    io,
    pin::{Pin, pin},
    rc::Rc,
    sync::Arc,
    task::{Context, Poll, Waker},
};

use incular_text::{FontLoader, FontLoaderError, TextAlign, TextEngine, TextStyle};

fn tick<F: Future>(future: Pin<&mut F>) -> Poll<F::Output> {
    future.poll(&mut Context::from_waker(Waker::noop()))
}

fn complete<F: Future>(future: F) -> F::Output {
    let mut future = pin!(future);
    match tick(future.as_mut()) {
        Poll::Ready(output) => output,
        Poll::Pending => panic!("this fixture contains no pending work"),
    }
}

type Loaded = Rc<RefCell<Vec<(Arc<str>, Vec<u8>)>>>;

fn register(
    trace: Loaded,
) -> impl FnMut(Arc<str>, Vec<u8>) -> std::future::Ready<Result<(), Infallible>> {
    move |family, bytes| {
        trace.borrow_mut().push((family, bytes));
        ready(Ok(()))
    }
}

#[test]
fn sends_the_exact_bytes_of_every_asset_under_the_same_family() {
    let mut loader = FontLoader::new("TestFamily");
    let expected = [vec![100], vec![10, 20, 30], vec![200]];
    for bytes in &expected {
        loader.add_bytes(bytes.clone()).unwrap();
    }
    let trace = Loaded::default();
    complete(loader.load_with(register(trace.clone()))).unwrap();
    let results = trace.borrow();
    assert_eq!(results.len(), 3);
    for ((family, bytes), expected) in results.iter().zip(expected) {
        assert_eq!(family.as_ref(), "TestFamily");
        assert_eq!(*bytes, expected);
    }
}

#[test]
fn a_ready_second_face_does_not_overtake_a_delayed_first_asset() {
    let mut loader = FontLoader::new("TestFamily");
    let ready_first = Rc::new(Cell::new(false));
    loader
        .add_font(poll_fn({
            let ready_first = ready_first.clone();
            move |_| {
                if ready_first.get() {
                    Poll::Ready(Ok::<_, Infallible>(vec![1]))
                } else {
                    Poll::Pending
                }
            }
        }))
        .unwrap();
    loader.add_bytes(vec![2]).unwrap();
    let trace = Loaded::default();
    {
        let mut load = pin!(loader.load_with(register(trace.clone())));
        assert!(tick(load.as_mut()).is_pending());
        assert!(trace.borrow().is_empty());
        ready_first.set(true);
        assert!(matches!(tick(load.as_mut()), Poll::Ready(Ok(()))));
    }
    assert_eq!(
        trace
            .borrow()
            .iter()
            .map(|(_, bytes)| bytes[0])
            .collect::<Vec<_>>(),
        [1, 2]
    );
    assert!(matches!(
        loader.add_bytes(vec![3]),
        Err(FontLoaderError::AlreadyLoaded)
    ));
    assert!(matches!(
        complete(loader.load()),
        Err(FontLoaderError::AlreadyLoaded)
    ));
}

#[test]
fn the_loader_awaits_registration_and_preserves_a_byte_subview() {
    static PADDED: &[u8] = &[80, 10, 20, 30, 90];
    let mut loader = FontLoader::new("Sliced");
    loader
        .add_font(ready(Ok::<_, Infallible>(&PADDED[1..4])))
        .unwrap();
    loader.add_bytes(vec![2]).unwrap();
    let trace = Loaded::default();
    let first_registered = Rc::new(Cell::new(false));
    let mut loading = pin!(loader.load_with({
        let first_registered = first_registered.clone();
        let trace = trace.clone();
        move |family, bytes| {
            let waits = bytes[0] == 10;
            trace.borrow_mut().push((family, bytes));
            let first_registered = first_registered.clone();
            poll_fn(move |_| {
                if waits && !first_registered.get() {
                    Poll::Pending
                } else {
                    Poll::Ready(Ok::<(), Infallible>(()))
                }
            })
        }
    }));
    assert!(tick(loading.as_mut()).is_pending());
    assert_eq!(
        trace.borrow().as_slice(),
        [(Arc::from("Sliced"), vec![10, 20, 30])]
    );
    first_registered.set(true);
    assert!(matches!(tick(loading.as_mut()), Poll::Ready(Ok(()))));
    assert_eq!(trace.borrow().len(), 2);
    assert_eq!(trace.borrow()[1].1, [2]);
}

#[test]
fn a_failed_asset_does_not_deliver_later_faces_or_allow_a_retry() {
    let mut loader = FontLoader::new("TestFamily");
    loader.add_bytes(vec![1]).unwrap();
    loader
        .add_font(ready(Err::<Vec<u8>, _>(io::Error::other("missing face"))))
        .unwrap();
    loader.add_bytes(vec![3]).unwrap();
    let trace = Loaded::default();
    let result = complete(loader.load_with(register(trace.clone())));
    assert!(
        matches!(result, Err(FontLoaderError::Asset(cause)) if cause.to_string() == "missing face")
    );
    assert_eq!(
        trace.borrow().as_slice(),
        [(Arc::from("TestFamily"), vec![1])]
    );
    assert!(matches!(
        complete(loader.load()),
        Err(FontLoaderError::AlreadyLoaded)
    ));
}

#[test]
fn loading_into_an_existing_engine_replaces_a_cached_font_fallback() {
    const FAMILY: &str = "Flutter 3.50 audit loaded face";
    let mut engine = TextEngine::new();
    let style = TextStyle::new().font_family(FAMILY).font_size(22.);
    let old = engine.layout("audit", &style, None, TextAlign::Start);
    let previous_generation = engine.font_database_generation();
    let mut bytes = include_bytes!("../../../harness/flutter/fonts/Inter-Regular.ttf").to_vec();
    // The trailer identifies the registered face even on an Inter-equipped host.
    bytes.extend_from_slice(b"ordered FontLoader fixture");
    let mut loader = FontLoader::new(FAMILY);
    loader.add_bytes(bytes.clone()).unwrap();
    complete(loader.load()).unwrap();

    let actual = engine.layout("audit", &style, None, TextAlign::Start);
    assert!(engine.font_database_generation() > previous_generation);
    assert!(!Arc::ptr_eq(&old, &actual));
    assert_eq!(actual.lines[0].runs[0].font.bytes().as_ref(), bytes);
}
