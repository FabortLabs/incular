//! Flutter's predictive back gesture (`services/predictive_back_event.dart`
//! and the `flutter/backgesture` channel handled by `WidgetsBinding`): the
//! progress of a system back swipe, reported while the user drags so the
//! current route can follow it before the back is committed or cancelled.

use std::{
    fmt,
    hash::{Hash, Hasher},
};

use incular_core::Offset;

use crate::message_codecs::{MessageValue, MethodCall};

/// Standard-method channel used for Android system back gesture stages.
pub const BACK_GESTURE_CHANNEL: &str = "flutter/backgesture";

/// A malformed entry in a predictive back platform message.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum BackGestureMessageError {
    /// The message did not supply a map where one was needed.
    #[error("predictive back arguments must be a map")]
    InvalidArguments,
    /// A non-null touch position must have two numeric components.
    #[error("touchOffset must be null or contain two numeric coordinates")]
    InvalidTouchOffset,
    /// Progress must be numeric and remain within the gesture's unit interval.
    #[error("progress must be a number from zero to one")]
    InvalidProgress,
    /// Unlike the native convenience constructor, the wire parser rejects
    /// unknown edge numbers rather than treating them as left.
    #[error("swipeEdge must be integer zero (left) or one (right)")]
    InvalidSwipeEdge,
    /// This channel does not handle the method.
    #[error("unknown predictive back method")]
    UnknownMethod,
}

/// Flutter's `SwipeEdge`: the screen edge a back swipe started from.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum SwipeEdge {
    /// The left edge.
    #[default]
    Left,
    /// The right edge.
    Right,
}

/// Flutter's `PredictiveBackEvent`: one sample of a back gesture.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PredictiveBackEvent {
    /// Where the finger is, in logical pixels, or `None` for a back button.
    pub touch_offset: Option<Offset>,
    /// How far the gesture has gone, from 0 to 1.
    pub progress: f32,
    /// The edge the swipe started from.
    pub swipe_edge: SwipeEdge,
}

impl PredictiveBackEvent {
    /// A convenience constructor for typed callers: clamps finite progress
    /// to `0..=1`. Use [`Self::try_from_message`] for strict wire validation.
    #[must_use]
    pub fn new(touch_offset: Option<Offset>, progress: f32, swipe_edge: SwipeEdge) -> Self {
        Self {
            touch_offset,
            progress: if progress.is_finite() {
                progress.clamp(0., 1.)
            } else {
                0.
            },
            swipe_edge,
        }
    }

    /// Android's `BackEventCompat` (`touchX`, `touchY`, `progress` and
    /// `swipeEdge`, where 0 is `EDGE_LEFT` and 1 `EDGE_RIGHT`), as Flutter's
    /// Android embedder encodes it. Touch positions are in physical pixels;
    /// `scale_factor` converts them to logical ones.
    #[must_use]
    pub fn from_android(
        touch_x: f32,
        touch_y: f32,
        progress: f32,
        swipe_edge: i32,
        scale_factor: f32,
    ) -> Self {
        let scale = if scale_factor.is_finite() && scale_factor > 0. {
            scale_factor
        } else {
            1.
        };
        let edge = if swipe_edge == 1 {
            SwipeEdge::Right
        } else {
            SwipeEdge::Left
        };
        Self::new(
            Some(Offset::new(touch_x / scale, touch_y / scale)),
            progress,
            edge,
        )
    }

    /// Decodes the map received from Flutter's standard back-gesture channel.
    /// Coordinates are already logical; this must not apply Android's pixel
    /// scale a second time. A missing or null position describes a button.
    pub fn try_from_message(message: &MessageValue) -> Result<Self, BackGestureMessageError> {
        use BackGestureMessageError as Error;
        let MessageValue::Map(entries) = message else {
            return Err(Error::InvalidArguments);
        };
        let field = |name: &str| {
            entries.iter().rev().find_map(|(key, value)| {
                matches!(key, MessageValue::String(key) if key == name).then_some(value)
            })
        };
        let touch_offset = match field("touchOffset") {
            None | Some(MessageValue::Null) => None,
            Some(value) => Some(decode_offset(value).ok_or(Error::InvalidTouchOffset)?),
        };
        let progress = field("progress")
            .and_then(number)
            .filter(|progress| (0.0..=1.0).contains(progress))
            .ok_or(Error::InvalidProgress)? as f32;
        let swipe_edge = match field("swipeEdge") {
            Some(MessageValue::Int(0)) => SwipeEdge::Left,
            Some(MessageValue::Int(1)) => SwipeEdge::Right,
            _ => return Err(Error::InvalidSwipeEdge),
        };
        Ok(Self {
            touch_offset,
            progress,
            swipe_edge,
        })
    }

    /// Flutter `isButtonEvent`: a back button press reported as a gesture,
    /// which has no touch position, or no progress and a zero position.
    #[must_use]
    pub fn is_button_event(&self) -> bool {
        self.touch_offset
            .is_none_or(|offset| self.progress == 0. && offset == Offset::ZERO)
    }
}

fn number(value: &MessageValue) -> Option<f64> {
    match value {
        MessageValue::Int(value) => Some(*value as f64),
        MessageValue::Float(value) => Some(*value),
        _ => None,
    }
}

fn decode_offset(value: &MessageValue) -> Option<Offset> {
    use MessageValue as V;
    let (x, y) = match value {
        V::List(items) => {
            let [x, y, ..] = items.as_slice() else {
                return None;
            };
            (number(x)? as f32, number(y)? as f32)
        }
        V::Bytes(items) => (f32::from(*items.first()?), f32::from(*items.get(1)?)),
        V::Int32List(items) => (*items.first()? as f32, *items.get(1)? as f32),
        V::Int64List(items) => (*items.first()? as f32, *items.get(1)? as f32),
        V::Float32List(items) => (*items.first()?, *items.get(1)?),
        V::Float64List(items) => (*items.first()? as f32, *items.get(1)? as f32),
        _ => return None,
    };
    Some(Offset::new(x, y))
}

impl Hash for PredictiveBackEvent {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let float = |value: f32| if value == 0. { 0 } else { value.to_bits() };
        self.touch_offset
            .map(|offset| (float(offset.x), float(offset.y)))
            .hash(state);
        float(self.progress).hash(state);
        self.swipe_edge.hash(state);
    }
}

impl fmt::Display for PredictiveBackEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("PredictiveBackEvent{touchOffset: ")?;
        match self.touch_offset {
            Some(point) => write!(f, "Offset({:.1}, {:.1})", point.x, point.y)?,
            None => f.write_str("null")?,
        }
        let edge = match self.swipe_edge {
            SwipeEdge::Left => "left",
            SwipeEdge::Right => "right",
        };
        write!(
            f,
            ", progress: {:?}, swipeEdge: SwipeEdge.{edge}}}",
            self.progress
        )
    }
}

/// The stages of a predictive back gesture, Flutter's `startBackGesture`,
/// `updateBackGestureProgress`, `commitBackGesture` and `cancelBackGesture`
/// messages (Android's `OnBackAnimationCallback` `onBackStarted`,
/// `onBackProgressed`, `onBackInvoked` and `onBackCancelled`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BackGestureEvent {
    /// The gesture started.
    Start(PredictiveBackEvent),
    /// The gesture moved.
    Update(PredictiveBackEvent),
    /// The gesture finished: the back happens.
    Commit,
    /// The gesture was abandoned: nothing happens.
    Cancel,
}

impl BackGestureEvent {
    /// Converts a decoded standard method call to the corresponding gesture
    /// event. Hosts can feed the result into their existing platform dispatch.
    pub fn try_from_method_call(call: &MethodCall) -> Result<Self, BackGestureMessageError> {
        use BackGestureMessageError as Error;
        if !matches!(call.arguments, MessageValue::Null | MessageValue::Map(_)) {
            return Err(Error::InvalidArguments);
        }
        match call.method.as_str() {
            "startBackGesture" => {
                PredictiveBackEvent::try_from_message(&call.arguments).map(Self::Start)
            }
            "updateBackGestureProgress" => {
                PredictiveBackEvent::try_from_message(&call.arguments).map(Self::Update)
            }
            "commitBackGesture" => Ok(Self::Commit),
            "cancelBackGesture" => Ok(Self::Cancel),
            _ => Err(Error::UnknownMethod),
        }
    }
}
