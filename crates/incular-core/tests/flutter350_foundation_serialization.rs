//! Pinned Flutter foundation/serialization_test.dart binary contracts.
use incular_core::serialization::{BufferError, Endian, ReadBuffer, WriteBuffer};
const VALUE: f64 = 314.0 / 100.0;

#[test]
fn empty_buffer_finalizes_to_no_bytes() {
    let bytes = WriteBuffer::new().done().unwrap();
    assert!(bytes.is_empty());
    assert!(!ReadBuffer::new(&bytes).has_remaining());
}

#[test]
fn single_byte_round_trips_unchanged() {
    let mut write = WriteBuffer::new();
    write.put_uint8(201).unwrap();
    let bytes = write.done().unwrap();
    assert_eq!(bytes, [201]);
    let mut read = ReadBuffer::new(&bytes);
    assert_eq!(read.get_uint8(), Ok(201));
    assert!(!read.has_remaining());
}

macro_rules! scalar_case {
    ($test:ident, $v:expr, $put:ident, $get:ident, $encoding:ident) => {
        #[test]
        fn $test() {
            let value = $v;
            let mut write = WriteBuffer::new();
            write.$put(value).unwrap();
            let bytes = write.done().unwrap();
            assert_eq!(bytes, value.$encoding());
            let mut read = ReadBuffer::new(&bytes);
            assert_eq!(read.$get(), Ok(value));
            assert!(!read.has_remaining());
        }
    };
}

macro_rules! big_case {
    ($test:ident, $v:expr, $put:ident, $get:ident) => {
        #[test]
        fn $test() {
            let value = $v;
            let mut write = WriteBuffer::new();
            write.$put(value, Endian::Big).unwrap();
            let bytes = write.done().unwrap();
            assert_eq!(bytes, value.to_be_bytes());
            let mut read = ReadBuffer::new(&bytes);
            assert_eq!(read.$get(Endian::Big), Ok(value));
            assert!(!read.has_remaining());
        }
    };
}

scalar_case!(
    int32_uses_host_order,
    -9_i32,
    put_int32,
    get_int32,
    to_ne_bytes
);
big_case!(
    int32_can_use_big_endian,
    -9_i32,
    put_int32_endian,
    get_int32_endian
);
scalar_case!(
    int64_uses_host_order,
    -9_000_000_000_000_i64,
    put_int64,
    get_int64,
    to_ne_bytes
);
big_case!(
    int64_can_use_big_endian,
    -9_000_000_000_000_i64,
    put_int64_endian,
    get_int64_endian
);
scalar_case!(
    float64_uses_host_order,
    VALUE,
    put_float64,
    get_float64,
    to_ne_bytes
);
big_case!(
    float64_can_use_big_endian,
    VALUE,
    put_float64_endian,
    get_float64_endian
);

macro_rules! padded_case {
    ($test:ident, $v:expr, $put:ident, $get:ident, $size:expr, $alignment:expr, $verify:expr) => {
        #[test]
        fn $test() {
            let values = $v;
            let mut write = WriteBuffer::with_capacity(1).unwrap();
            write.put_uint8(9).unwrap();
            write.$put(&values).unwrap();
            let bytes = write.done().unwrap();
            assert_eq!(bytes.len(), $size);
            assert_eq!(bytes[0], 9);
            assert!(bytes[1..$alignment].iter().all(|byte| *byte == 0));
            let mut read = ReadBuffer::new(&bytes);
            assert_eq!(read.get_uint8(), Ok(9));
            let actual = read.$get(values.len()).unwrap();
            ($verify)(actual, values);
            assert!(!read.has_remaining());
        }
    };
}

padded_case!(
    unaligned_int32_list_adds_four_byte_padding,
    [-99_i32, 2, 99],
    put_int32_list,
    get_int32_list,
    16,
    4,
    |actual: Vec<i32>, values| assert_eq!(actual, values)
);
padded_case!(
    unaligned_int64_list_adds_eight_byte_padding,
    [-99_i64, 2, 99],
    put_int64_list,
    get_int64_list,
    32,
    8,
    |actual: Vec<i64>, values| assert_eq!(actual, values)
);
padded_case!(
    unaligned_float32_list_preserves_value_and_nan,
    [VALUE as f32, f32::NAN],
    put_float32_list,
    get_float32_list,
    12,
    4,
    |actual: Vec<f32>, values: [f32; 2]| {
        assert_eq!(actual[0], values[0]);
        assert!(actual[1].is_nan());
    }
);
padded_case!(
    unaligned_float64_list_preserves_value_and_nan,
    [VALUE, f64::NAN],
    put_float64_list,
    get_float64_list,
    24,
    8,
    |actual: Vec<f64>, values: [f64; 2]| {
        assert_eq!(actual[0], values[0]);
        assert!(actual[1].is_nan());
    }
);

#[test]
fn writer_rejects_a_second_finalization_and_later_writes() {
    let mut write = WriteBuffer::new();
    assert!(write.done().is_ok());
    assert_eq!(write.done(), Err(BufferError::Finalized));
    assert_eq!(write.put_uint8(3), Err(BufferError::Finalized));
}

#[test]
fn starting_capacity_zero_is_rejected() {
    assert!(matches!(
        WriteBuffer::with_capacity(0),
        Err(BufferError::ZeroCapacity)
    ));
}

#[test]
fn starting_capacity_one_is_supported() {
    let mut write = WriteBuffer::with_capacity(1).unwrap();
    write.put_uint8_list(&[1, 2, 3]).unwrap();
    assert_eq!(write.done().unwrap(), [1, 2, 3]);
}

#[test]
fn scalar_alignment_differs_from_integer_alignment() {
    let mut write = WriteBuffer::with_capacity(1).unwrap();
    write.put_uint8(7).unwrap();
    write.put_uint16_endian(0x0102, Endian::Big).unwrap();
    write.put_uint32_endian(0x03040506, Endian::Little).unwrap();
    write.put_float64_endian(0.5, Endian::Big).unwrap();
    let bytes = write.done().unwrap();
    assert_eq!(&bytes[..8], &[7, 1, 2, 6, 5, 4, 3, 0]);
    assert_eq!(&bytes[8..], &0.5_f64.to_be_bytes());
    let mut read = ReadBuffer::new(&bytes);
    assert_eq!(read.get_uint8().unwrap(), 7);
    assert_eq!(read.get_uint16_endian(Endian::Big).unwrap(), 0x0102);
    assert_eq!(read.get_uint32_endian(Endian::Little).unwrap(), 0x03040506);
    assert_eq!(read.get_float64_endian(Endian::Big).unwrap(), 0.5);
}

#[test]
fn truncated_or_overflowing_requests_do_not_advance_the_cursor() {
    let mut read = ReadBuffer::new(&[1, 2, 3]);
    assert_eq!(
        read.get_int32_list(usize::MAX),
        Err(BufferError::LengthOverflow)
    );
    assert_eq!(
        read.get_float64(),
        Err(BufferError::Truncated { end: 8, length: 3 })
    );
    assert_eq!(read.get_uint8_list(3), Ok(&[1_u8, 2, 3][..]));
    assert!(!read.has_remaining());
}
