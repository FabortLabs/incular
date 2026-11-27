//! The Flutter engine's small StandardMethodCodec control-channel subset.

use std::fmt;

pub(super) enum Control<'a> {
    Resize { channel: &'a str, capacity: usize },
    Overflow { channel: &'a str, expected: bool },
}

/// Invalid control data leaves all channel buffers untouched.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FlutterChannelControlError {
    Malformed,
    UnknownMethod,
    InvalidChannel,
    InvalidCapacity,
}

impl fmt::Display for FlutterChannelControlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Malformed => "malformed Flutter channel buffer control message",
            Self::UnknownMethod => "unknown Flutter channel buffer control method",
            Self::InvalidChannel => "invalid Flutter channel buffer name",
            Self::InvalidCapacity => "Flutter channel buffer capacity is outside 0..=2147483647",
        })
    }
}

impl std::error::Error for FlutterChannelControlError {}

fn byte(data: &mut &[u8]) -> Result<u8, FlutterChannelControlError> {
    let (tag, rest) = data
        .split_first()
        .ok_or(FlutterChannelControlError::Malformed)?;
    *data = rest;
    Ok(*tag)
}

fn string<'a>(data: &mut &'a [u8]) -> Result<&'a str, FlutterChannelControlError> {
    if byte(data)? != 7 {
        return Err(FlutterChannelControlError::Malformed);
    }
    let size = usize::from(byte(data)?);
    if size >= 254 {
        return Err(FlutterChannelControlError::Malformed);
    }
    let (value, rest) = data
        .split_at_checked(size)
        .ok_or(FlutterChannelControlError::Malformed)?;
    let value = std::str::from_utf8(value).map_err(|_| FlutterChannelControlError::Malformed)?;
    *data = rest;
    Ok(value)
}

fn channel(name: &str) -> Result<&str, FlutterChannelControlError> {
    if name.contains('\0') {
        Err(FlutterChannelControlError::InvalidChannel)
    } else {
        Ok(name)
    }
}

pub(super) fn parse(mut data: &[u8]) -> Result<Control<'_>, FlutterChannelControlError> {
    use FlutterChannelControlError as Error;
    if data.first() != Some(&7) {
        let legacy = std::str::from_utf8(data).map_err(|_| Error::Malformed)?;
        let mut fields = legacy.split('\r');
        if let (Some("resize"), Some(name), Some(capacity), None) =
            (fields.next(), fields.next(), fields.next(), fields.next())
        {
            let capacity = capacity
                .trim()
                .parse::<i32>()
                .ok()
                .and_then(|value| usize::try_from(value).ok())
                .ok_or(Error::InvalidCapacity)?;
            return Ok(Control::Resize {
                channel: channel(name)?,
                capacity,
            });
        }
        return Err(Error::Malformed);
    }
    let method = string(&mut data)?;
    if !matches!(method, "resize" | "overflow") {
        return Err(Error::UnknownMethod);
    }
    if byte(&mut data)? != 12 || byte(&mut data)? < 2 {
        return Err(Error::Malformed);
    }
    let target = channel(string(&mut data)?)?;
    if method == "resize" {
        if byte(&mut data)? != 3 {
            return Err(Error::InvalidCapacity);
        }
        let raw = data.first_chunk::<4>().ok_or(Error::Malformed)?;
        let capacity =
            usize::try_from(i32::from_ne_bytes(*raw)).map_err(|_| Error::InvalidCapacity)?;
        Ok(Control::Resize {
            channel: target,
            capacity,
        })
    } else {
        let expected = match byte(&mut data)? {
            1 => true,
            2 => false,
            _ => return Err(Error::Malformed),
        };
        Ok(Control::Overflow {
            channel: target,
            expected,
        })
    }
}
