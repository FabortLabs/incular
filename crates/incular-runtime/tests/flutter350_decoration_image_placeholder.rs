use std::{cell::RefCell, rc::Rc};

use incular_config::Constraints;
use incular_core::Size;
use incular_image::{ImageError, ImageHandle, WeakImageHandle};
use incular_rendering::PaintCommand;
use incular_runtime::Runtime;
use incular_widgets::{
    AnyImageProvider, BoxDecoration, DecoratedBox, DecorationImage, ImageConfiguration,
    ImageProvider, ImageStream, SizedBox, Widget,
};

#[derive(Clone, Debug)]
struct Factory {
    name: &'static str,
    size: u32,
    resolutions: Rc<RefCell<Vec<&'static str>>>,
    weak_frames: Rc<RefCell<Vec<WeakImageHandle>>>,
}

impl PartialEq for Factory {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && Rc::ptr_eq(&self.weak_frames, &other.weak_frames)
    }
}

impl Eq for Factory {}

impl Factory {
    fn new(name: &'static str, size: u32, resolutions: Rc<RefCell<Vec<&'static str>>>) -> Self {
        Self {
            name,
            size,
            resolutions,
            weak_frames: Rc::default(),
        }
    }
}

fn rgba(size: u32) -> ImageHandle {
    ImageHandle::from_rgba8(size, size, vec![255; (size * size * 4) as usize]).unwrap()
}

impl ImageProvider for Factory {
    fn load(&self) -> Result<ImageHandle, ImageError> {
        panic!("a painter must observe the provider's stream")
    }

    fn resolve(&self, _: &ImageConfiguration) -> ImageStream {
        self.resolutions.borrow_mut().push(self.name);
        let bitmap = rgba(self.size);
        self.weak_frames.borrow_mut().push(bitmap.downgrade());
        ImageStream::ready(bitmap)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Delayed(ImageStream);

impl ImageProvider for Delayed {
    fn load(&self) -> Result<ImageHandle, ImageError> {
        panic!("a painter must observe the provider's stream")
    }
    fn resolve(&self, _: &ImageConfiguration) -> ImageStream {
        self.0.clone()
    }
}

fn mount(image: DecorationImage) -> Runtime {
    Runtime::new(
        DecoratedBox::new(SizedBox::shrink())
            .decoration(BoxDecoration::new().image(image))
            .into(),
    )
    .unwrap()
}

fn draw_sizes(runtime: &mut Runtime) -> Vec<(u32, f32)> {
    let list = runtime
        .run_frame(Constraints::tight(Size::new(100., 100.)))
        .unwrap()
        .0;
    list.commands()
        .iter()
        .filter_map(|command| match command {
            PaintCommand::Image {
                image, destination, ..
            } => Some((image.decoded().width(), destination.size.width)),
            _ => None,
        })
        .collect()
}

#[test]
fn a_pending_decoration_paints_then_retires_its_placeholder() {
    let resolutions = Rc::new(RefCell::new(Vec::new()));
    let placeholder = Factory::new("placeholder", 10, resolutions.clone());
    let watched = placeholder.weak_frames.clone();
    let (stream, completion) = ImageStream::loading();
    let mut runtime = mount(
        DecorationImage::from_provider(Delayed(stream))
            .placeholder(AnyImageProvider::new(placeholder)),
    );
    assert_eq!(draw_sizes(&mut runtime), vec![(10, 10.)]);
    assert!(!runtime.frame_requested());
    assert_eq!(&*resolutions.borrow(), &["placeholder"]);
    assert!(!watched.borrow()[0].is_dropped());

    assert!(completion.complete(rgba(100)));
    assert!(runtime.frame_requested());
    assert_eq!(draw_sizes(&mut runtime), vec![(100, 100.)]);
    assert!(!runtime.frame_requested());
    assert_eq!(&*resolutions.borrow(), &["placeholder"]);
    assert!(watched.borrow()[0].is_dropped());
}

#[test]
fn a_synchronously_ready_primary_never_paints_the_placeholder() {
    let resolutions = Rc::new(RefCell::new(Vec::new()));
    let placeholder = Factory::new("placeholder", 10, resolutions.clone());
    let watched = placeholder.weak_frames.clone();
    let primary = Factory::new("primary", 100, resolutions.clone());
    let mut runtime = mount(
        DecorationImage::from_provider(primary).placeholder(AnyImageProvider::new(placeholder)),
    );
    assert_eq!(draw_sizes(&mut runtime), vec![(100, 100.)]);
    assert_eq!(&*resolutions.borrow(), &["placeholder", "primary"]);
    assert!(watched.borrow()[0].is_dropped());
    assert!(!runtime.frame_requested());
}

#[test]
fn unmounting_a_decoration_releases_the_painter_owned_image() {
    let primary = Factory::new("primary", 100, Rc::default());
    let watched = primary.weak_frames.clone();
    let scene = |child: Widget| SizedBox::new().width(100.).height(100.).child(child).into();
    let painted = DecoratedBox::new(SizedBox::shrink())
        .decoration(BoxDecoration::new().image(DecorationImage::from_provider(primary)));
    let mut runtime = Runtime::new(scene(painted.into())).unwrap();
    assert_eq!(draw_sizes(&mut runtime), vec![(100, 100.)]);
    assert!(!watched.borrow()[0].is_dropped());
    let root = runtime.tree().root().unwrap();
    let next = scene(SizedBox::shrink().into());
    runtime.schedule_update(root, next).unwrap();
    assert!(draw_sizes(&mut runtime).is_empty());
    assert!(watched.borrow()[0].is_dropped());
}

#[test]
fn placeholder_configuration_participates_in_equality_and_diagnostics() {
    let primary = Factory::new("primary", 100, Rc::default());
    let fallback = Factory::new("placeholder", 10, Rc::default());
    let plain = DecorationImage::from_provider(primary);
    let illustrated = plain.clone().placeholder(AnyImageProvider::new(fallback));
    assert_ne!(illustrated, plain);
    assert_eq!(illustrated.clone(), illustrated);
    let rendered = format!("{illustrated:?}");
    assert!(
        rendered.contains("placeholder: Some(Provider(Factory { name: \"placeholder\""),
        "{rendered}"
    );
}
