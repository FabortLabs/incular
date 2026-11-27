use std::{cell::RefCell, collections::BTreeMap, rc::Rc};

use incular_config::{Alignment, Constraints};
use incular_core::Size;
use incular_image::{ImageError, ImageHandle, ImageId, ImageStream, ImageStreamCompleter};
use incular_rendering::PaintCommand;
use incular_runtime::Runtime;
use incular_widgets::{
    Align, BoxDecoration, BoxFit, DecoratedBox, DecorationImage, ImageConfiguration, ImageProvider,
    SizedBox, Widget,
};

type Pending = Rc<RefCell<BTreeMap<u32, (ImageStream, ImageStreamCompleter)>>>;

#[derive(Clone)]
struct SizedProvider(Pending);

impl std::fmt::Debug for SizedProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SizedProvider")
    }
}

impl PartialEq for SizedProvider {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for SizedProvider {}

impl ImageProvider for SizedProvider {
    fn load(&self) -> Result<ImageHandle, ImageError> {
        panic!("a decoration must use the provider's size-specific stream");
    }
    fn resolve(&self, config: &ImageConfiguration) -> ImageStream {
        let width = config.size.unwrap().width.round() as u32;
        self.0
            .borrow_mut()
            .entry(width)
            .or_insert_with(ImageStream::loading)
            .0
            .clone()
    }
}

fn bitmap(color: [u8; 4]) -> ImageHandle {
    ImageHandle::from_rgba8(1, 1, color.to_vec()).unwrap()
}

fn scene(width: f32, decoration: &BoxDecoration) -> Widget {
    Align::new(
        Alignment::TOP_LEFT,
        SizedBox::new()
            .width(width)
            .height(16.)
            .child(DecoratedBox::new(SizedBox::shrink()).decoration(decoration.clone())),
    )
    .into()
}

fn draws(runtime: &mut Runtime) -> Vec<(ImageId, Size)> {
    let list = runtime
        .run_frame(Constraints::tight(Size::new(200., 100.)))
        .unwrap()
        .0;
    list.commands()
        .iter()
        .filter_map(|command| match command {
            PaintCommand::Image {
                image, destination, ..
            } => Some((image.id(), destination.size)),
            _ => None,
        })
        .collect()
}

fn finish(streams: &Pending, width: u32, image: &ImageHandle) {
    assert!(streams.borrow()[&width].1.complete(image.clone()));
}

#[test]
fn an_unchanged_resized_decoration_keeps_the_last_primary_until_its_replacement_is_ready() {
    for use_fallback in [true, false] {
        let streams = Pending::default();
        let mut image = DecorationImage::from_provider(SizedProvider(streams.clone()));
        image.fit = Some(BoxFit::Fill);
        let fallback = bitmap([0, 255, 0, 255]);
        if use_fallback {
            image = image.placeholder(fallback.clone());
        }
        let decoration = BoxDecoration::new().image(image);
        let mut runtime = Runtime::new(scene(40., &decoration)).unwrap();
        let root = runtime.tree().root().unwrap();
        let initially = draws(&mut runtime);
        if use_fallback {
            assert_eq!(initially, [(fallback.id(), Size::new(40., 16.))]);
        } else {
            assert!(initially.is_empty());
        }

        let first = bitmap([255, 0, 0, 255]);
        finish(&streams, 40, &first);
        assert_eq!(draws(&mut runtime), [(first.id(), Size::new(40., 16.))]);

        runtime
            .schedule_update(root, scene(80., &decoration))
            .unwrap();
        assert_eq!(
            draws(&mut runtime),
            [(first.id(), Size::new(80., 16.))],
            "placeholder present: {use_fallback}"
        );
        assert_eq!(runtime.tree().root(), Some(root));
        assert!(streams.borrow().contains_key(&80));

        let second = bitmap([0, 0, 255, 255]);
        finish(&streams, 80, &second);
        assert_eq!(draws(&mut runtime), [(second.id(), Size::new(80., 16.))]);

        runtime
            .schedule_update(root, scene(100., &decoration))
            .unwrap();
        assert_eq!(draws(&mut runtime), [(second.id(), Size::new(100., 16.))]);
        runtime
            .schedule_update(root, scene(120., &decoration))
            .unwrap();
        assert_eq!(draws(&mut runtime), [(second.id(), Size::new(120., 16.))]);
        finish(&streams, 100, &first);
        assert_eq!(draws(&mut runtime), [(second.id(), Size::new(120., 16.))]);
        finish(&streams, 120, &first);
        assert_eq!(draws(&mut runtime), [(first.id(), Size::new(120., 16.))]);
    }
}
