//! Flutter 3.50.0 services/predictive_back_event_test.dart, through the standard method codec.

use incular_core::Offset;
use incular_platform::{
    BACK_GESTURE_CHANNEL, BackGestureEvent, BackGestureMessageError, PredictiveBackEvent,
    SwipeEdge,
    message_codecs::{MessageValue as V, MethodCall, StandardMethodCodec},
};
use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};

fn message(position: V, progress: V, edge: V) -> V {
    V::Map(vec![
        (V::String("touchOffset".into()), position),
        (V::String("progress".into()), progress),
        (V::String("swipeEdge".into()), edge),
    ])
}

fn point(x: f64, y: f64) -> V {
    V::List(vec![V::Float(x), V::Float(y)])
}

fn standard_call(method: &str, arguments: V) -> MethodCall {
    let call = MethodCall {
        method: method.into(),
        arguments,
    };
    StandardMethodCodec::decode_method_call(
        &StandardMethodCodec::encode_method_call(&call).unwrap(),
    )
    .unwrap()
}

fn decode(
    position: V,
    progress: V,
    edge: V,
) -> Result<PredictiveBackEvent, BackGestureMessageError> {
    let call = standard_call("startBackGesture", message(position, progress, edge));
    match BackGestureEvent::try_from_method_call(&call)? {
        BackGestureEvent::Start(event) => Ok(event),
        _ => unreachable!("a start method must decode as start"),
    }
}

#[test]
fn left_and_right_wire_edges_keep_nonzero_positions_at_zero_progress() {
    for (encoded, edge) in [(0, SwipeEdge::Left), (1, SwipeEdge::Right)] {
        let event = decode(point(0., 100.), V::Float(0.), V::Int(encoded)).unwrap();
        assert_eq!(event.swipe_edge, edge);
        assert_eq!(event.touch_offset, Some(Offset::new(0., 100.)));
        assert_eq!(event.progress, 0.);
        assert!(!event.is_button_event());
    }
}

#[test]
fn both_null_and_zero_position_are_back_buttons_at_zero_progress() {
    for (position, expected) in [(V::Null, None), (point(0., 0.), Some(Offset::ZERO))] {
        let event = decode(position, V::Float(0.), V::Int(1)).unwrap();
        assert_eq!(event.touch_offset, expected);
        assert!(event.is_button_event());
    }
}

#[test]
fn invalid_progress_is_not_clamped_when_it_comes_from_the_wire() {
    for value in [2., -0.01, f64::NAN, f64::INFINITY, 1.00000001] {
        assert_eq!(
            decode(point(0., 100.), V::Float(value), V::Int(1)),
            Err(BackGestureMessageError::InvalidProgress)
        );
    }
}

#[test]
fn invalid_edges_are_not_coerced_to_left() {
    for edge in [V::Int(2), V::Int(-1), V::Float(1.), V::Null] {
        assert_eq!(
            decode(point(0., 100.), V::Float(0.), edge),
            Err(BackGestureMessageError::InvalidSwipeEdge)
        );
    }
}

fn hash(value: PredictiveBackEvent) -> u64 {
    let mut state = DefaultHasher::new();
    value.hash(&mut state);
    state.finish()
}

#[test]
fn wire_events_use_structural_equality_hashing_and_display() {
    let event = |x| decode(point(x, 100.), V::Float(0.), V::Int(0)).unwrap();
    let a = event(0.);
    let b = event(0.);
    let moved = event(1.);
    assert_eq!(a, b);
    assert_eq!(hash(a), hash(b));
    assert_eq!(a.to_string(), b.to_string());
    assert_eq!(
        a.to_string(),
        "PredictiveBackEvent{touchOffset: Offset(0.0, 100.0), progress: 0.0, swipeEdge: SwipeEdge.left}"
    );
    assert_ne!(a, moved);
    assert_ne!(hash(a), hash(moved));
    assert_ne!(a.to_string(), moved.to_string());

    let negative_zero = event(-0.);
    assert_eq!(a, negative_zero);
    assert_eq!(hash(a), hash(negative_zero));
}

#[test]
fn numeric_coordinate_lists_and_null_arguments_follow_channel_boundaries() {
    assert_eq!(BACK_GESTURE_CHANNEL, "flutter/backgesture");
    let args = message(V::Float64List(vec![2., 50.]), V::Int(1), V::Int(0));
    let update = standard_call("updateBackGestureProgress", args);
    let BackGestureEvent::Update(event) = BackGestureEvent::try_from_method_call(&update).unwrap()
    else {
        panic!("update event");
    };
    assert_eq!(event.touch_offset, Some(Offset::new(2., 50.)));
    assert_eq!(event.progress, 1.);
    for (method, expected) in [
        ("commitBackGesture", BackGestureEvent::Commit),
        ("cancelBackGesture", BackGestureEvent::Cancel),
    ] {
        assert_eq!(
            BackGestureEvent::try_from_method_call(&standard_call(method, V::Null)),
            Ok(expected)
        );
    }
    assert_eq!(
        BackGestureEvent::try_from_method_call(&standard_call("startBackGesture", V::Null)),
        Err(BackGestureMessageError::InvalidArguments),
    );
    assert_eq!(
        BackGestureEvent::try_from_method_call(&standard_call("unknown", V::Null)),
        Err(BackGestureMessageError::UnknownMethod),
    );
    assert_eq!(
        decode(V::List(vec![V::Int(1)]), V::Float(0.), V::Int(1)),
        Err(BackGestureMessageError::InvalidTouchOffset)
    );
}
