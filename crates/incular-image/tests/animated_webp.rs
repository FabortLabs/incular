use std::{sync::Arc, time::Duration};

use incular_image::{AnimatedWebpDecoder, DecodedImageFrame, ImageError};

const FINITE: &[u8] = include_bytes!("fixtures/webp_two_cycles.webp");
const PARTIAL: &[u8] = include_bytes!("fixtures/webp_partial_frames.webp");
const STILL: &[u8] = include_bytes!("fixtures/webp_still.webp");
const RED: [u8; 4] = [255, 0, 0, 255];
const BLUE: [u8; 4] = [0, 0, 255, 255];
const CLEAR: [u8; 4] = [0; 4];

fn decoder(bytes: &[u8]) -> AnimatedWebpDecoder {
    AnimatedWebpDecoder::from_bytes(Arc::from(bytes))
        .unwrap()
        .unwrap()
}

fn pixel(frame: &DecodedImageFrame, x: usize, y: usize, color: [u8; 4]) {
    let image = frame.image.image().decoded();
    let offset = (y * image.width() as usize + x) * 4;
    assert_eq!(&image.pixels()[offset..offset + 4], &color);
}

#[test]
fn finite_metadata_and_composited_pixel_order_match_the_real_bitstream() {
    let mut source = decoder(FINITE);
    assert_eq!(source.frame_count().get(), 2);
    assert_eq!(source.repetition_count(), Some(1));
    for (millis, expected) in [(200, RED), (400, BLUE), (200, RED), (400, BLUE)] {
        let frame = source.decode_next().unwrap();
        assert_eq!(frame.duration, Duration::from_millis(millis));
        assert_eq!(frame.image.logical_size(), (4.0, 4.0));
        for y in 0..4 {
            for x in 0..4 {
                pixel(&frame, x, y, expected);
            }
        }
    }
}

#[test]
fn a_fresh_cycle_discards_the_previous_loops_partial_canvas() {
    let mut source = decoder(PARTIAL);
    assert_eq!(source.frame_count().get(), 2);
    assert_eq!(source.repetition_count(), None);
    for right in [CLEAR, BLUE, CLEAR, BLUE] {
        let frame = source.decode_next().unwrap();
        assert_eq!(frame.image.logical_size(), (4.0, 2.0));
        for y in 0..2 {
            for x in 0..2 {
                pixel(&frame, x, y, RED);
                pixel(&frame, x + 2, y, right);
            }
        }
    }
}

#[test]
fn still_or_bad_data_does_not_create_an_animation_decoder() {
    assert!(
        AnimatedWebpDecoder::from_bytes(Arc::from(STILL))
            .unwrap()
            .is_none()
    );
    assert!(matches!(
        AnimatedWebpDecoder::from_bytes(Arc::from(&b"not webp"[..])),
        Err(ImageError::Decode(_))
    ));
}

#[test]
fn disposal_clears_only_the_previous_frame_rectangle() {
    let mut bytes = PARTIAL.to_vec();
    let first = bytes.windows(4).position(|chunk| chunk == b"ANMF").unwrap();
    bytes[first + 8 + 15] |= 1;
    let mut source = AnimatedWebpDecoder::from_bytes(bytes.into())
        .unwrap()
        .unwrap();
    let first = source.decode_next().unwrap();
    pixel(&first, 0, 0, RED);
    pixel(&first, 3, 1, CLEAR);
    let second = source.decode_next().unwrap();
    pixel(&second, 0, 0, CLEAR);
    pixel(&second, 3, 1, BLUE);
}

#[test]
fn oversized_canvas_is_rejected_before_allocating_a_frame() {
    let mut bytes = FINITE.to_vec();
    bytes[24..27].copy_from_slice(&16_384_u32.to_le_bytes()[..3]);
    assert!(matches!(
        AnimatedWebpDecoder::from_bytes(bytes.into()),
        Err(ImageError::DecodeTooLarge {
            width: 16_385,
            height: 4
        })
    ));
}

#[test]
fn decoding_can_run_in_a_blocking_pool_without_moving_ui_state() {
    fn require_send<T: Send>() {}
    require_send::<AnimatedWebpDecoder>();
    require_send::<DecodedImageFrame>();
}
