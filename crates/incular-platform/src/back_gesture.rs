//! Flutter's predictive back gesture (`services/predictive_back_event.dart`
//! and the `flutter/backgesture` channel handled by `WidgetsBinding`): the
//! progress of a system back swipe, reported while the user drags so the
//! current route can follow it before the back is committed or cancelled.

use incular_core::Offset;

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
    /// An event with `progress` clamped to `0..=1`, as Flutter asserts.
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

    /// Flutter `isButtonEvent`: a back button press reported as a gesture,
    /// which has no touch position, or no progress and a zero position.
    #[must_use]
    pub fn is_button_event(&self) -> bool {
        self.touch_offset
            .is_none_or(|offset| self.progress == 0. && offset == Offset::ZERO)
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
