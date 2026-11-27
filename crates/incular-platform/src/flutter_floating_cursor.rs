//! The platform message describing a keyboard-driven floating caret.

use incular_core::Offset;
use serde_json::Value;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FloatingCursorDragState {
    Start,
    Update,
    End,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FlutterFloatingCursorPoint {
    pub state: FloatingCursorDragState,
    pub offset: Offset,
}

impl FlutterFloatingCursorPoint {
    pub fn from_flutter_json(
        state: &str,
        position: &Value,
    ) -> Result<Self, FlutterFloatingCursorPointError> {
        use FlutterFloatingCursorPointError::*;
        let state = match state {
            "FloatingCursorDragState.start" => FloatingCursorDragState::Start,
            "FloatingCursorDragState.update" => FloatingCursorDragState::Update,
            "FloatingCursorDragState.end" => FloatingCursorDragState::End,
            _ => return Err(UnknownState),
        };
        let position = position.as_object().ok_or(InvalidPosition)?;
        let coordinate = |key| {
            position
                .get(key)
                .filter(|coordinate| !coordinate.is_null())
                .ok_or(MissingCoordinate(key))
        };
        let (x, y) = (coordinate("X")?, coordinate("Y")?);
        let offset = if state == FloatingCursorDragState::Update {
            let number = |key, value: &Value| {
                value
                    .as_f64()
                    .map(|value| value as f32)
                    .filter(|value| value.is_finite())
                    .ok_or(InvalidCoordinate(key))
            };
            Offset::new(number("X", x)?, number("Y", y)?)
        } else {
            Offset::ZERO
        };
        Ok(Self { state, offset })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum FlutterFloatingCursorPointError {
    #[error("unknown Flutter floating cursor state")]
    UnknownState,
    #[error("expected a floating cursor position object")]
    InvalidPosition,
    #[error("missing floating cursor coordinate {0}")]
    MissingCoordinate(&'static str),
    #[error("floating cursor coordinate {0} is not a finite number")]
    InvalidCoordinate(&'static str),
}
