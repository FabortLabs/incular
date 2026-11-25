//! Sequential binary buffers with Flutter's host-endian alignment rules.

/// Byte order for scalar values. Typed lists always use the host's order.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Endian {
    #[default]
    Host,
    Little,
    Big,
}

/// A binary buffer operation could not be completed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum BufferError {
    #[error("the write buffer has already been finalized")]
    Finalized,
    #[error("the write buffer needs a positive starting capacity")]
    ZeroCapacity,
    #[error("the requested buffer offset or length overflowed")]
    LengthOverflow,
    #[error("reading through byte {end} exceeds the {length}-byte input")]
    Truncated { end: usize, length: usize },
}

/// A one-shot writer. Integer scalars can be unaligned; float64 values and
/// typed lists get zero padding to the corresponding element alignment.
pub struct WriteBuffer {
    bytes: Option<Vec<u8>>,
}

impl Default for WriteBuffer {
    fn default() -> Self {
        Self::new()
    }
}

macro_rules! put_scalar {
    ($method:ident, $ordered:ident, $type:ty, $alignment:expr) => {
        pub fn $method(&mut self, value: $type) -> Result<(), BufferError> {
            self.$ordered(value, Endian::Host)
        }

        pub fn $ordered(&mut self, value: $type, endian: Endian) -> Result<(), BufferError> {
            let bytes = match endian {
                Endian::Host => value.to_ne_bytes(),
                Endian::Little => value.to_le_bytes(),
                Endian::Big => value.to_be_bytes(),
            };
            self.write_aligned(&bytes, $alignment)
        }
    };
}

macro_rules! put_list {
    ($method:ident, $type:ty, $alignment:expr) => {
        pub fn $method(&mut self, values: &[$type]) -> Result<(), BufferError> {
            let size = values
                .len()
                .checked_mul($alignment)
                .ok_or(BufferError::LengthOverflow)?;
            let bytes = self.aligned_output($alignment, size)?;
            for value in values {
                bytes.extend_from_slice(&value.to_ne_bytes());
            }
            Ok(())
        }
    };
}

impl WriteBuffer {
    #[must_use]
    pub fn new() -> Self {
        Self {
            bytes: Some(Vec::with_capacity(8)),
        }
    }

    pub fn with_capacity(start_capacity: usize) -> Result<Self, BufferError> {
        if start_capacity == 0 {
            return Err(BufferError::ZeroCapacity);
        }
        Ok(Self {
            bytes: Some(Vec::with_capacity(start_capacity)),
        })
    }

    /// Returns the written bytes without copying. Further writes and calls to
    /// `done` fail.
    pub fn done(&mut self) -> Result<Vec<u8>, BufferError> {
        self.bytes.take().ok_or(BufferError::Finalized)
    }

    pub fn put_uint8(&mut self, value: u8) -> Result<(), BufferError> {
        self.write_aligned(&[value], 1)
    }

    put_scalar!(put_uint16, put_uint16_endian, u16, 1);
    put_scalar!(put_uint32, put_uint32_endian, u32, 1);
    put_scalar!(put_int32, put_int32_endian, i32, 1);
    put_scalar!(put_int64, put_int64_endian, i64, 1);
    put_scalar!(put_float64, put_float64_endian, f64, 8);

    pub fn put_uint8_list(&mut self, values: &[u8]) -> Result<(), BufferError> {
        self.write_aligned(values, 1)
    }

    put_list!(put_int32_list, i32, 4);
    put_list!(put_int64_list, i64, 8);
    put_list!(put_float32_list, f32, 4);
    put_list!(put_float64_list, f64, 8);

    fn write_aligned(&mut self, data: &[u8], alignment: usize) -> Result<(), BufferError> {
        self.aligned_output(alignment, data.len())?
            .extend_from_slice(data);
        Ok(())
    }

    fn aligned_output(
        &mut self,
        alignment: usize,
        data_len: usize,
    ) -> Result<&mut Vec<u8>, BufferError> {
        let bytes = self.bytes.as_mut().ok_or(BufferError::Finalized)?;
        let aligned = align_up(bytes.len(), alignment)?;
        let end = aligned
            .checked_add(data_len)
            .ok_or(BufferError::LengthOverflow)?;
        bytes.reserve(end - bytes.len());
        bytes.resize(aligned, 0);
        Ok(bytes)
    }
}

fn align_up(position: usize, alignment: usize) -> Result<usize, BufferError> {
    let padding = position.wrapping_neg() & (alignment - 1);
    position
        .checked_add(padding)
        .ok_or(BufferError::LengthOverflow)
}

/// A checked, sequential reader over borrowed bytes. Failed reads do not
/// advance the cursor. Typed lists decode to owned, naturally aligned Rust
/// values; a list of bytes is borrowed.
pub struct ReadBuffer<'a> {
    data: &'a [u8],
    position: usize,
}

macro_rules! get_scalar {
    ($method:ident, $ordered:ident, $type:ty, $alignment:expr, $size:expr) => {
        pub fn $method(&mut self) -> Result<$type, BufferError> {
            self.$ordered(Endian::Host)
        }

        pub fn $ordered(&mut self, endian: Endian) -> Result<$type, BufferError> {
            let bytes: [u8; $size] = self
                .take($size, $alignment)?
                .try_into()
                .expect("a checked read returns exactly the requested length");
            Ok(match endian {
                Endian::Host => <$type>::from_ne_bytes(bytes),
                Endian::Little => <$type>::from_le_bytes(bytes),
                Endian::Big => <$type>::from_be_bytes(bytes),
            })
        }
    };
}

macro_rules! get_list {
    ($method:ident, $type:ty, $alignment:expr) => {
        pub fn $method(&mut self, count: usize) -> Result<Vec<$type>, BufferError> {
            let size = count
                .checked_mul($alignment)
                .ok_or(BufferError::LengthOverflow)?;
            Ok(self
                .take(size, $alignment)?
                .chunks_exact($alignment)
                .map(|chunk| {
                    <$type>::from_ne_bytes(
                        chunk
                            .try_into()
                            .expect("the chunk length equals the element width"),
                    )
                })
                .collect())
        }
    };
}

impl<'a> ReadBuffer<'a> {
    #[must_use]
    pub const fn new(data: &'a [u8]) -> Self {
        Self { data, position: 0 }
    }

    #[must_use]
    pub const fn has_remaining(&self) -> bool {
        self.remaining() != 0
    }

    /// Bytes available starting at the current cursor.
    #[must_use]
    pub const fn remaining(&self) -> usize {
        self.data.len() - self.position
    }

    pub fn get_uint8(&mut self) -> Result<u8, BufferError> {
        Ok(self.take(1, 1)?[0])
    }

    get_scalar!(get_uint16, get_uint16_endian, u16, 1, 2);
    get_scalar!(get_uint32, get_uint32_endian, u32, 1, 4);
    get_scalar!(get_int32, get_int32_endian, i32, 1, 4);
    get_scalar!(get_int64, get_int64_endian, i64, 1, 8);
    get_scalar!(get_float64, get_float64_endian, f64, 8, 8);

    pub fn get_uint8_list(&mut self, count: usize) -> Result<&'a [u8], BufferError> {
        self.take(count, 1)
    }

    get_list!(get_int32_list, i32, 4);
    get_list!(get_int64_list, i64, 8);
    get_list!(get_float32_list, f32, 4);
    get_list!(get_float64_list, f64, 8);

    fn take(&mut self, count: usize, alignment: usize) -> Result<&'a [u8], BufferError> {
        let start = align_up(self.position, alignment)?;
        let end = start
            .checked_add(count)
            .ok_or(BufferError::LengthOverflow)?;
        let data = self.data.get(start..end).ok_or(BufferError::Truncated {
            end,
            length: self.data.len(),
        })?;
        self.position = end;
        Ok(data)
    }
}
