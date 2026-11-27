//! Flutter 3.50.0 painting/image_stream_test.dart:1184 and :1211.
use std::{cell::RefCell, rc::Rc};

use incular_image::{ImageError, ImageLoadState, ImageStream, ImageStreamListener};

fn verify_error_after_listener_removal(report_errors: bool) {
    let (stream, completer) = ImageStream::loading();
    let reported = Rc::new(RefCell::new(Vec::new()));
    let trace = reported.clone();
    completer.set_unhandled_error_reporter(move |error| trace.borrow_mut().push(error));
    let listener = ImageStreamListener::new(Rc::new(|_, _| panic!("no decoded image")));
    let listener = if report_errors {
        listener
    } else {
        listener
            .on_error(Rc::new(|_| panic!("the listener is already removed")))
            .report_errors(false)
    };
    let subscription = stream.listen(listener);
    drop(subscription);

    let error = ImageError::Decode("test error".into());
    assert!(completer.fail(error.clone()));
    assert_eq!(stream.state(), ImageLoadState::Failed(error.clone()));
    assert_eq!(
        reported.take(),
        if report_errors { vec![error] } else { vec![] }
    );
    assert!(!completer.fail(ImageError::Decode("repeated completion".into())));
    assert!(reported.borrow().is_empty());
}

#[test]
fn errors_are_reported_after_listener_removal_by_default() {
    verify_error_after_listener_removal(true);
}

#[test]
fn a_removed_listener_can_permanently_suppress_unhandled_reports() {
    verify_error_after_listener_removal(false);
}

#[test]
fn a_live_error_handler_receives_the_failure_instead_of_the_reporter() {
    let (stream, completer) = ImageStream::loading();
    completer.set_unhandled_error_reporter(|_| panic!("the listener handles this error"));
    let seen = Rc::new(RefCell::new(Vec::new()));
    let trace = seen.clone();
    let _subscription = stream.listen(
        ImageStreamListener::new(Rc::new(|_, _| panic!("no decoded image")))
            .on_error(Rc::new(move |error| trace.borrow_mut().push(error))),
    );
    let error = ImageError::Decode("handled error".into());
    assert!(completer.fail(error.clone()));
    assert_eq!(seen.take(), [error]);
}

#[test]
fn suppression_survives_a_pending_completer_handoff() {
    let (stream, handoff) = ImageStream::loading();
    let active = stream.listen(ImageStreamListener::new(Rc::new(|_, _| {})).report_errors(false));
    drop(active);
    let (source, completer) = ImageStream::loading();
    completer.set_unhandled_error_reporter(|_| panic!("the followed stream remembers the policy"));
    assert!(handoff.follow(&source));
    let error = ImageError::Decode("after handoff".into());
    assert!(completer.fail(error.clone()));
    assert_eq!(stream.state(), ImageLoadState::Failed(error));
}

#[test]
fn report_errors_is_part_of_the_listener_identity() {
    let default = ImageStreamListener::new(Rc::new(|_, _| {}));
    assert_eq!(default, default.clone().report_errors(true));
    assert_ne!(default, default.clone().report_errors(false));
}
