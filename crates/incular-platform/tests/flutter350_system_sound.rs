use std::{cell::RefCell, rc::Rc};

use incular_platform::{SystemSound, SystemSoundType};

#[test]
fn system_sound_requests_reach_only_the_current_host() {
    let service = SystemSound::default();
    let events = Rc::new(RefCell::new(Vec::new()));
    service.play(SystemSoundType::Alert); // unsupported before binding
    service.bind_host({
        let events = events.clone();
        move |sound| events.borrow_mut().push(sound)
    });
    let held_by_widget = service.clone();
    service.play(SystemSoundType::Click);
    held_by_widget.play(SystemSoundType::Tick);
    service.play(SystemSoundType::Alert);
    assert_eq!(
        &*events.borrow(),
        &[
            SystemSoundType::Click,
            SystemSoundType::Tick,
            SystemSoundType::Alert
        ],
    );
    service.unbind_host();
    held_by_widget.play(SystemSoundType::Alert);
    assert_eq!(events.borrow().len(), 3);

    // A host may tear itself down while handling the request.
    service.bind_host({
        let service = service.clone();
        let events = events.clone();
        move |sound| {
            events.borrow_mut().push(sound);
            service.unbind_host();
        }
    });
    held_by_widget.play(SystemSoundType::Click);
    held_by_widget.play(SystemSoundType::Tick);
    assert_eq!(events.borrow().last(), Some(&SystemSoundType::Click));
    assert_eq!(events.borrow().len(), 4);
}
