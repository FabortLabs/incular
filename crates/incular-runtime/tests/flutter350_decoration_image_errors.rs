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
