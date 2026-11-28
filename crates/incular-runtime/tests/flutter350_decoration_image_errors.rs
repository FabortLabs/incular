use std::{cell::RefCell, rc::Rc};

use incular_config::Constraints;
use incular_core::Size;
use incular_image::{ImageError, ImageHandle, ImageStream};
use incular_runtime::Runtime;
use incular_widgets::{
    BoxDecoration, DecoratedBox, DecorationImage, ImageConfiguration, ImageProvider, SizedBox,
    Widget,
};

#[derive(Clone, Debug, PartialEq, Eq)]
struct Provider(ImageStream);

impl ImageProvider for Provider {
    fn load(&self) -> Result<ImageHandle, ImageError> {
        panic!("a mounted decoration must resolve the stream");
    }

    fn resolve(&self, _: &ImageConfiguration) -> ImageStream {
        self.0.clone()
    }
}

fn scene(stream: ImageStream, report: impl Fn(ImageError) + 'static) -> Widget {
    let image = DecorationImage::from_provider(Provider(stream)).on_error(report);
    DecoratedBox::new(SizedBox::shrink())
        .decoration(BoxDecoration::new().image(image))
        .into()
}

fn frame(runtime: &mut Runtime) {
    runtime
        .run_frame(Constraints::tight(Size::new(100., 100.)))
        .unwrap();
}

#[test]
fn synchronously_failed_decoration_delivers_the_original_error_after_materializing() {
    let recorded = Rc::new(RefCell::new(Vec::new()));
    let observed = recorded.clone();
    let error = ImageError::Decode("threw".into());
    let mut runtime = Runtime::new(scene(ImageStream::failed(error.clone()), move |error| {
        observed.borrow_mut().push(error);
    }))
    .unwrap();
    assert!(recorded.borrow().is_empty());
    frame(&mut runtime);
    assert_eq!(&*recorded.borrow(), &[error.clone()]);
    frame(&mut runtime);
    assert_eq!(&*recorded.borrow(), &[error]);
}

#[test]
fn a_retired_decoration_stream_cannot_call_the_old_error_handler() {
    let recorded = Rc::new(RefCell::new(Vec::new()));
    let first_recorded = recorded.clone();
    let (first, first_completion) = ImageStream::loading();
    let mut runtime = Runtime::new(scene(first, move |error| {
        first_recorded.borrow_mut().push(("old", error))
    }))
    .unwrap();
    frame(&mut runtime);
    let root = runtime.tree().root().unwrap();
    let next_recorded = recorded.clone();
    let (next, next_completion) = ImageStream::loading();
    runtime
        .schedule_update(
            root,
            scene(next, move |error| {
                next_recorded.borrow_mut().push(("new", error))
            }),
        )
        .unwrap();
    frame(&mut runtime);

    assert!(first_completion.fail(ImageError::Decode("stale".into())));
    assert!(recorded.borrow().is_empty());
    let error = ImageError::Io("current".into());
    assert!(next_completion.fail(error.clone()));
    frame(&mut runtime);
    assert_eq!(&*recorded.borrow(), &[("new", error)]);
}

#[test]
fn paint_equivalent_decoration_images_ignore_error_handler_identity() {
    use incular_widgets::{Border, ShapeDecoration};
    use std::hash::{DefaultHasher, Hash, Hasher};

    let (stream, _completion) = ImageStream::loading();
    let provider = Provider(stream);
    let a = DecorationImage::from_provider(provider.clone()).on_error(|_| panic!("not mounted"));
    let b = DecorationImage::from_provider(provider).on_error(|_| {});
    let mut c = b.clone();
    c.opacity = 0.5;
    assert_eq!(a, b);
    assert_ne!(a, c);
    assert_eq!(
        BoxDecoration::new().image(a.clone()),
        BoxDecoration::new().image(b.clone())
    );
    assert_ne!(
        BoxDecoration::new().image(a.clone()),
        BoxDecoration::new().image(c.clone())
    );

    let shape = |image| ShapeDecoration::new(Border::default()).image(image);
    let hash = |shape: &ShapeDecoration| {
        let mut state = DefaultHasher::new();
        shape.hash(&mut state);
        state.finish()
    };
    let first = shape(a);
    let equivalent = shape(b);
    assert!(first == equivalent);
    assert_eq!(hash(&first), hash(&equivalent));
    assert!(first != shape(c));
}

#[test]
fn callback_only_updates_keep_the_stream_but_deliver_its_failure_to_the_current_handler() {
    use incular_widgets::{Border, DecoratedShapeBox, ShapeDecoration};
    use std::cell::Cell;

    #[derive(Clone, Debug)]
    struct Held {
        stream: ImageStream,
        resolves: Rc<Cell<usize>>,
    }
    impl PartialEq for Held {
        fn eq(&self, other: &Self) -> bool {
            self.stream == other.stream
        }
    }
    impl Eq for Held {}
    impl ImageProvider for Held {
        fn load(&self) -> Result<ImageHandle, ImageError> {
            panic!("held provider must resolve")
        }
        fn resolve(&self, _: &ImageConfiguration) -> ImageStream {
            self.resolves.set(self.resolves.get() + 1);
            self.stream.clone()
        }
    }

    for shape in [false, true] {
        for change_paint in [false, true] {
            let (stream, completion) = ImageStream::loading();
            let provider = Held {
                stream,
                resolves: Rc::default(),
            };
            let seen = Rc::new(RefCell::new(Vec::new()));
            let scene = |listener, opacity| {
                let seen = seen.clone();
                let mut image =
                    DecorationImage::from_provider(provider.clone()).on_error(move |error| {
                        seen.borrow_mut().push((listener, error));
                    });
                image.opacity = opacity;
                if shape {
                    DecoratedShapeBox::new(
                        ShapeDecoration::new(Border::default()).image(image),
                        SizedBox::shrink(),
                    )
                    .into()
                } else {
                    DecoratedBox::new(SizedBox::shrink())
                        .decoration(BoxDecoration::new().image(image))
                        .into()
                }
            };
            let mut runtime = Runtime::new(scene("old", 1.)).unwrap();
            frame(&mut runtime);
            runtime
                .schedule_update(
                    runtime.tree().root().unwrap(),
                    scene("new", if change_paint { 0.5 } else { 1.0 }),
                )
                .unwrap();
            frame(&mut runtime);
            assert_eq!(
                provider.resolves.get(),
                1,
                "do not reopen an unchanged provider flight"
            );
            assert!(seen.borrow().is_empty());

            let error = ImageError::Decode("held image failed".into());
            assert!(completion.fail(error.clone()));
            frame(&mut runtime);
            assert_eq!(&*seen.borrow(), &[("new", error)]);
        }
    }
}
