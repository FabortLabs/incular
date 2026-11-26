//! Flutter-compatible binary values shared by packaged assets and platform channels.

use crate::serialization::{BufferError, ReadBuffer, WriteBuffer};
use std::fmt;

const NULL: u8 = 0;
const TRUE: u8 = 1;
const FALSE: u8 = 2;
const INT32: u8 = 3;
const INT64: u8 = 4;
const LARGE_INT_TEXT: u8 = 5;
const FLOAT64: u8 = 6;
const STRING: u8 = 7;
const BYTES: u8 = 8;
const INT32_LIST: u8 = 9;
const INT64_LIST: u8 = 10;
const FLOAT64_LIST: u8 = 11;
const LIST: u8 = 12;
const MAP: u8 = 13;
const FLOAT32_LIST: u8 = 14;

const MAX_DEPTH: usize = 128;

/// Values carried by the standard message codec.
///
/// Map entries retain wire order and support non-string keys, including
/// null. This is a wire value, not a Rust hash map.
#[derive(Clone, Debug, PartialEq)]
pub enum MessageValue {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Bytes(Vec<u8>),
    Int32List(Vec<i32>),
    Int64List(Vec<i64>),
    Float32List(Vec<f32>),
    Float64List(Vec<f64>),
    List(Vec<MessageValue>),
    Map(Vec<(MessageValue, MessageValue)>),
}

impl fmt::Display for MessageValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Null => f.write_str("null"),
            Self::Bool(value) => value.fmt(f),
            Self::Int(value) => value.fmt(f),
            Self::Float(value) if *value == f64::INFINITY => f.write_str("Infinity"),
            Self::Float(value) if *value == f64::NEG_INFINITY => f.write_str("-Infinity"),
            Self::Float(value) => write!(f, "{value:?}"),
            Self::String(value) => f.write_str(value),
            _ => write!(f, "{self:?}"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum MessageCodecError {
    #[error(transparent)]
    Buffer(#[from] BufferError),
    #[error("unknown standard message type {0}")]
    UnknownType(u8),
    #[error("the encoded message contains invalid UTF-8")]
    InvalidUtf8,
    #[error("the encoded message has trailing bytes")]
    TrailingBytes,
    #[error("the message collection length exceeds the wire format or available bytes")]
    InvalidLength,
    #[error("the message exceeds the supported nesting depth")]
    TooDeep,
}

/// The standard binary message codec. Platform channels exchange these
/// bytes in host byte order, not a network-endian wire format.
#[derive(Clone, Copy, Debug, Default)]
pub struct StandardMessageCodec;

impl StandardMessageCodec {
    /// A top-level null is represented by no message bytes. Null inside a
    /// collection is encoded with its own discriminator.
    pub fn encode_message(message: &MessageValue) -> Result<Option<Vec<u8>>, MessageCodecError> {
        if matches!(message, MessageValue::Null) {
            return Ok(None);
        }
        let mut buffer = WriteBuffer::new();
        Self::write_value(&mut buffer, message, 0)?;
        Ok(Some(buffer.done()?))
    }

    /// Decodes one value and rejects trailing bytes. Nesting is limited to
    /// 128 levels to keep the decoder's stack use bounded.
    pub fn decode_message(bytes: Option<&[u8]>) -> Result<MessageValue, MessageCodecError> {
        let Some(bytes) = bytes else {
            return Ok(MessageValue::Null);
        };
        let mut buffer = ReadBuffer::new(bytes);
        let value = Self::read_value(&mut buffer, 0)?;
        if buffer.has_remaining() {
            return Err(MessageCodecError::TrailingBytes);
        }
        Ok(value)
    }

    fn write_value(
        buffer: &mut WriteBuffer,
        value: &MessageValue,
        depth: usize,
    ) -> Result<(), MessageCodecError> {
        if depth > MAX_DEPTH {
            return Err(MessageCodecError::TooDeep);
        }
        match value {
            MessageValue::Null => buffer.put_uint8(NULL)?,
            MessageValue::Bool(value) => buffer.put_uint8(if *value { TRUE } else { FALSE })?,
            MessageValue::Int(value) => {
                if let Ok(narrow) = i32::try_from(*value) {
                    buffer.put_uint8(INT32)?;
                    buffer.put_int32(narrow)?;
                } else {
                    buffer.put_uint8(INT64)?;
                    buffer.put_int64(*value)?;
                }
            }
            MessageValue::Float(value) => {
                buffer.put_uint8(FLOAT64)?;
                buffer.put_float64(*value)?;
            }
            MessageValue::String(value) => write_string(buffer, value)?,
            MessageValue::Bytes(value) => {
                write_prefix(buffer, BYTES, value.len())?;
                buffer.put_uint8_list(value)?;
            }
            MessageValue::Int32List(value) => {
                write_prefix(buffer, INT32_LIST, value.len())?;
                buffer.put_int32_list(value)?;
            }
            MessageValue::Int64List(value) => {
                write_prefix(buffer, INT64_LIST, value.len())?;
                buffer.put_int64_list(value)?;
            }
            MessageValue::Float32List(value) => {
                write_prefix(buffer, FLOAT32_LIST, value.len())?;
                buffer.put_float32_list(value)?;
            }
            MessageValue::Float64List(value) => {
                write_prefix(buffer, FLOAT64_LIST, value.len())?;
                buffer.put_float64_list(value)?;
            }
            MessageValue::List(values) => {
                write_prefix(buffer, LIST, values.len())?;
                for value in values {
                    Self::write_value(buffer, value, depth + 1)?;
                }
            }
            MessageValue::Map(entries) => {
                write_prefix(buffer, MAP, entries.len())?;
                for (key, value) in entries {
                    Self::write_value(buffer, key, depth + 1)?;
                    Self::write_value(buffer, value, depth + 1)?;
                }
            }
        }
        Ok(())
    }

    fn read_value(
        buffer: &mut ReadBuffer<'_>,
        depth: usize,
    ) -> Result<MessageValue, MessageCodecError> {
        use MessageValue as V;
        if depth > MAX_DEPTH {
            return Err(MessageCodecError::TooDeep);
        }
        Ok(match buffer.get_uint8()? {
            NULL => V::Null,
            TRUE => V::Bool(true),
            FALSE => V::Bool(false),
            INT32 => V::Int(buffer.get_int32()?.into()),
            INT64 => V::Int(buffer.get_int64()?),
            FLOAT64 => V::Float(buffer.get_float64()?),
            STRING | LARGE_INT_TEXT => {
                let length = read_size(buffer)?;
                let text = std::str::from_utf8(buffer.get_uint8_list(length)?)
                    .map_err(|_| MessageCodecError::InvalidUtf8)?;
                V::String(text.to_owned())
            }
            BYTES => {
                let length = read_size(buffer)?;
                V::Bytes(buffer.get_uint8_list(length)?.to_vec())
            }
            INT32_LIST => {
                let length = read_size(buffer)?;
                V::Int32List(buffer.get_int32_list(length)?)
            }
            INT64_LIST => {
                let length = read_size(buffer)?;
                V::Int64List(buffer.get_int64_list(length)?)
            }
            FLOAT32_LIST => {
                let length = read_size(buffer)?;
                V::Float32List(buffer.get_float32_list(length)?)
            }
            FLOAT64_LIST => {
                let length = read_size(buffer)?;
                V::Float64List(buffer.get_float64_list(length)?)
            }
            LIST => {
                let length = checked_slots(buffer, 1)?;
                let mut values = Vec::with_capacity(length.min(64));
                for _ in 0..length {
                    values.push(Self::read_value(buffer, depth + 1)?);
                }
                V::List(values)
            }
            MAP => {
                let length = checked_slots(buffer, 2)?;
                let mut entries = Vec::with_capacity(length.min(64));
                for _ in 0..length {
                    let key = Self::read_value(buffer, depth + 1)?;
                    let value = Self::read_value(buffer, depth + 1)?;
                    entries.push((key, value));
                }
                V::Map(entries)
            }
            unknown => return Err(MessageCodecError::UnknownType(unknown)),
        })
    }
}

fn write_prefix(buffer: &mut WriteBuffer, tag: u8, length: usize) -> Result<(), MessageCodecError> {
    let length = u32::try_from(length).map_err(|_| MessageCodecError::InvalidLength)?;
    buffer.put_uint8(tag)?;
    match length {
        0..=253 => buffer.put_uint8(length as u8)?,
        254..=65_535 => {
            buffer.put_uint8(254)?;
            buffer.put_uint16(length as u16)?;
        }
        _ => {
            buffer.put_uint8(255)?;
            buffer.put_uint32(length)?;
        }
    }
    Ok(())
}

fn read_size(buffer: &mut ReadBuffer<'_>) -> Result<usize, MessageCodecError> {
    let size = match buffer.get_uint8()? {
        254 => buffer.get_uint16()?.into(),
        255 => buffer.get_uint32()?,
        small => small.into(),
    };
    usize::try_from(size).map_err(|_| MessageCodecError::InvalidLength)
}

fn checked_slots(
    buffer: &mut ReadBuffer<'_>,
    tags_per_slot: usize,
) -> Result<usize, MessageCodecError> {
    let length = read_size(buffer)?;
    // Every value consumes at least one tag byte, before any allocation
    // for the declared number of list or map entries.
    if length > buffer.remaining() / tags_per_slot {
        return Err(MessageCodecError::InvalidLength);
    }
    Ok(length)
}

/// A callable platform method and its argument, which may be null.
#[derive(Clone, Debug, PartialEq)]
pub struct MethodCall<Value = MessageValue> {
    pub method: String,
    pub arguments: Value,
}

/// An error replied to a method call by the platform.
#[derive(Clone, Debug, PartialEq)]
pub struct PlatformException<Value = MessageValue> {
    pub code: String,
    pub message: Option<String>,
    pub details: Value,
    pub stacktrace: Option<String>,
}

impl<Value: fmt::Display> fmt::Display for MethodCall<Value> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "MethodCall({}, {})", self.method, self.arguments)
    }
}

impl<Value: fmt::Display> fmt::Display for PlatformException<Value> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "PlatformException({}, {}, {}, {})",
            self.code,
            self.message.as_deref().unwrap_or("null"),
            self.details,
            self.stacktrace.as_deref().unwrap_or("null"),
        )
    }
}
impl<Value: fmt::Debug + fmt::Display> std::error::Error for PlatformException<Value> {}

/// No plugin supplied an implementation for a requested method.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MissingPluginException {
    pub message: Option<String>,
}

impl fmt::Display for MissingPluginException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "MissingPluginException({})",
            self.message.as_deref().unwrap_or("null")
        )
    }
}
impl std::error::Error for MissingPluginException {}

#[derive(Debug, PartialEq, thiserror::Error)]
pub enum MethodCodecError {
    #[error(transparent)]
    Message(#[from] MessageCodecError),
    #[error("the encoded method call is malformed")]
    InvalidMethodCall,
    #[error("the encoded reply envelope is malformed")]
    InvalidEnvelope,
    #[error(transparent)]
    Platform(Box<PlatformException>),
}

impl From<BufferError> for MethodCodecError {
    fn from(error: BufferError) -> Self {
        MessageCodecError::from(error).into()
    }
}

/// Flutter's standard layout for platform method calls and reply envelopes.
#[derive(Clone, Copy, Debug, Default)]
pub struct StandardMethodCodec;

impl StandardMethodCodec {
    pub fn encode_method_call(call: &MethodCall) -> Result<Vec<u8>, MethodCodecError> {
        let mut buffer = WriteBuffer::new();
        write_string(&mut buffer, &call.method)?;
        StandardMessageCodec::write_value(&mut buffer, &call.arguments, 0)?;
        Ok(buffer.done()?)
    }

    pub fn decode_method_call(data: &[u8]) -> Result<MethodCall, MethodCodecError> {
        let mut buffer = ReadBuffer::new(data);
        let method = StandardMessageCodec::read_value(&mut buffer, 0)?;
        let arguments = StandardMessageCodec::read_value(&mut buffer, 0)?;
        let MessageValue::String(method) = method else {
            return Err(MethodCodecError::InvalidMethodCall);
        };
        if buffer.has_remaining() {
            return Err(MethodCodecError::InvalidMethodCall);
        }
        Ok(MethodCall { method, arguments })
    }

    pub fn encode_success_envelope(value: &MessageValue) -> Result<Vec<u8>, MethodCodecError> {
        let mut buffer = WriteBuffer::new();
        buffer.put_uint8(0)?;
        StandardMessageCodec::write_value(&mut buffer, value, 0)?;
        Ok(buffer.done()?)
    }

    pub fn encode_error_envelope(error: &PlatformException) -> Result<Vec<u8>, MethodCodecError> {
        let mut buffer = WriteBuffer::new();
        buffer.put_uint8(1)?;
        write_string(&mut buffer, &error.code)?;
        if let Some(message) = &error.message {
            write_string(&mut buffer, message)?;
        } else {
            buffer.put_uint8(NULL)?;
        }
        StandardMessageCodec::write_value(&mut buffer, &error.details, 0)?;
        if let Some(stacktrace) = &error.stacktrace {
            write_string(&mut buffer, stacktrace)?;
        }
        Ok(buffer.done()?)
    }

    pub fn decode_envelope(data: &[u8]) -> Result<MessageValue, MethodCodecError> {
        if data.is_empty() {
            return Err(MethodCodecError::InvalidEnvelope);
        }
        let mut buffer = ReadBuffer::new(data);
        if buffer.get_uint8()? == 0 {
            // Flutter permits additional bytes after the successful value.
            return Ok(StandardMessageCodec::read_value(&mut buffer, 0)?);
        }
        let MessageValue::String(code) = StandardMessageCodec::read_value(&mut buffer, 0)? else {
            return Err(MethodCodecError::InvalidEnvelope);
        };
        let message = read_optional_string(&mut buffer)?;
        let details = StandardMessageCodec::read_value(&mut buffer, 0)?;
        let stacktrace = if buffer.has_remaining() {
            read_optional_string(&mut buffer)?
        } else {
            None
        };
        if buffer.has_remaining() {
            return Err(MethodCodecError::InvalidEnvelope);
        }
        Err(MethodCodecError::Platform(Box::new(PlatformException {
            code,
            message,
            details,
            stacktrace,
        })))
    }
}

fn write_string(buffer: &mut WriteBuffer, value: &str) -> Result<(), MessageCodecError> {
    write_prefix(buffer, STRING, value.len())?;
    buffer.put_uint8_list(value.as_bytes())?;
    Ok(())
}

fn read_optional_string(buffer: &mut ReadBuffer<'_>) -> Result<Option<String>, MethodCodecError> {
    match StandardMessageCodec::read_value(buffer, 0)? {
        MessageValue::String(value) => Ok(Some(value)),
        MessageValue::Null => Ok(None),
        _ => Err(MethodCodecError::InvalidEnvelope),
    }
}
