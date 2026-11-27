use std::{cell::RefCell, rc::Rc};

use incular_image::{ImageHandle, ImageInfo, ImageStream};

fn original_image() -> ImageHandle {
    ImageHandle::from_rgba8(20, 10, vec![0; 20 * 10 * 4]).unwrap()
}

#[test]
fn image_info_clones_compare_original_resource_scale_and_label_without_copying() {
    let image = original_image();
    let weak = image.downgrade();
    let native = ImageInfo::new(image.clone());
    let scaled = ImageInfo::new(image.clone()).with_scale(2.);
    let other = ImageInfo::new(image.clone()).with_scale(2.);
    assert!(!native.is_clone_of(&scaled));
    assert!(scaled.is_clone_of(&other));
    assert!(!scaled.is_clone_of(&ImageInfo::new(original_image()).with_scale(2.)));
    assert!(!scaled.is_clone_of(&other.clone().with_debug_label("variant")));
    assert_eq!(native.logical_size(), (20., 10.));
    assert_eq!(scaled.logical_size(), (10., 5.));
    assert_eq!(scaled.size_bytes(), 800);

    let original = scaled.with_debug_label("2x");
    let cloned = original.clone();
    assert_eq!(cloned.debug_label(), Some("2x"));
    assert!(cloned.is_clone_of(&original));
    drop((native, other, image, original));
    assert!(!weak.is_dropped());
    drop(cloned);
    assert!(weak.is_dropped());
}

#[cfg(debug_assertions)]
#[test]
fn image_info_reports_its_own_create_and_dispose_events() {
    use incular_core::memory::allocations::{MemoryAllocations, ObjectEventKind};

    let events = Rc::new(RefCell::new(Vec::new()));
    let _listener = MemoryAllocations::current().add_listener({
        let events = events.clone();
        move |event| {
            if event.class_name == "ImageInfo" {
                events.borrow_mut().push(event);
            }
        }
    });
    let info = ImageInfo::new(original_image());
    assert_eq!(events.borrow().len(), 1);
    drop(info);
    let events = events.borrow();
    assert_eq!(
        events.iter().map(|event| event.kind).collect::<Vec<_>>(),
        [ObjectEventKind::Created, ObjectEventKind::Disposed]
    );
    assert_eq!(events[0].id, events[1].id);
    assert!(events.iter().all(|event| event.library == "painting"));
}

#[test]
fn ready_handoffs_report_whether_the_listener_preceded_the_attachment() {
    for attach_first in [true, false] {
        let image = original_image();
        let (target, completer) = ImageStream::loading();
        let source = ImageStream::ready(image.clone());
        let events = Rc::new(RefCell::new(Vec::new()));
        if attach_first {
            assert!(completer.follow(&source));
        }
        let _listener = target.listen_frame({
            let events = events.clone();
            move |result, synchronous| events.borrow_mut().push((result.unwrap(), synchronous))
        });
        if !attach_first {
            assert!(events.borrow().is_empty());
            assert!(completer.follow(&source));
        }
        assert_eq!(*events.borrow(), [(image, attach_first)]);
    }
}
