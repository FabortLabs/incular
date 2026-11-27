use std::{
    io::{self, BufRead, Cursor, Read, Seek, SeekFrom},
    num::NonZeroUsize,
    sync::Arc,
    time::Duration,
};

use image_webp::{DecodingError, LoopCount, WebPDecoder};

use crate::{DecodedImageFrame, ImageError, ImageHandle, ImageInfo, MAX_DECODE_OUTPUT_BYTES};

/// Sequential, renderer-neutral decoder for a compressed WebP animation.
/// Keeps compressed bytes and a compositing canvas, not all decoded frames.
/// Do expensive decoding on the host's blocking pool.
pub struct AnimatedWebpDecoder {
    bytes: Arc<[u8]>,
    frames: Vec<FrameChunk>,
    frame_count: NonZeroUsize,
    repetition_count: Option<usize>,
    canvas: Vec<u8>,
    width: u32,
    height: u32,
    next_index: usize,
}

struct FrameChunk {
    start: usize,
    end: usize,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    duration: Duration,
    blend: bool,
    dispose: bool,
}

fn u24(bytes: &[u8]) -> u32 {
    u32::from_le_bytes([bytes[0], bytes[1], bytes[2], 0])
}

fn malformed() -> ImageError {
    ImageError::Decode("invalid animated WebP frame container".into())
}

fn decode_error(error: DecodingError, width: u32, height: u32) -> ImageError {
    match error {
        DecodingError::ImageTooLarge | DecodingError::MemoryLimitExceeded => {
            ImageError::DecodeTooLarge { width, height }
        }
        error => ImageError::Decode(error.to_string()),
    }
}

impl AnimatedWebpDecoder {
    /// Reads animation metadata without decoding pixels. Returns `None`
    /// for a valid, still WebP instead of creating a playback decoder.
    pub fn from_bytes(bytes: Arc<[u8]>) -> Result<Option<Self>, ImageError> {
        let decoder = WebPDecoder::new(Cursor::new(bytes.as_ref()))
            .map_err(|error| decode_error(error, 0, 0))?;
        let (width, height) = decoder.dimensions();
        ImageHandle::check_decode_policy(width, height)?;
        if !decoder.is_animated() {
            return Ok(None);
        }
        let frame_count = NonZeroUsize::new(decoder.num_frames() as usize).ok_or_else(malformed)?;
        let repetition_count = match decoder.loop_count() {
            LoopCount::Forever => None,
            LoopCount::Times(plays) => Some(usize::from(plays.get()) - 1),
        };
        let mut frames = Vec::new();
        let mut offset = 12;
        while let Some(header) = bytes.get(offset..).and_then(|tail| tail.get(..8)) {
            let size =
                u32::from_le_bytes(header[4..8].try_into().expect("four-byte size")) as usize;
            let start = offset + 8;
            let end = start.checked_add(size).ok_or_else(malformed)?;
            if &header[..4] == b"ANMF" {
                let data = bytes
                    .get(start..end)
                    .filter(|data| data.len() >= 24)
                    .ok_or_else(malformed)?;
                let x = u24(&data[0..3]) * 2;
                let y = u24(&data[3..6]) * 2;
                let frame_width = u24(&data[6..9]) + 1;
                let frame_height = u24(&data[9..12]) + 1;
                if x + frame_width > width || y + frame_height > height {
                    return Err(malformed());
                }
                frames.push(FrameChunk {
                    start: start + 16,
                    end,
                    x,
                    y,
                    width: frame_width,
                    height: frame_height,
                    duration: Duration::from_millis(u64::from(u24(&data[12..15]))),
                    blend: data[15] & 2 == 0,
                    dispose: data[15] & 1 != 0,
                });
            }
            offset = end.checked_add(size & 1).ok_or_else(malformed)?;
        }
        if frames.len() != frame_count.get() {
            return Err(malformed());
        }
        Ok(Some(Self {
            bytes,
            frames,
            frame_count,
            repetition_count,
            canvas: Vec::new(),
            width,
            height,
            next_index: 0,
        }))
    }

    #[must_use]
    pub fn frame_count(&self) -> NonZeroUsize {
        self.frame_count
    }

    /// Number of repeats after the first play. `None` is infinite.
    #[must_use]
    pub fn repetition_count(&self) -> Option<usize> {
        self.repetition_count
    }

    /// Decodes a single fully composited RGBA image. Each new loop starts
    /// with a clear canvas. The caller controls when to stop according to
    /// `repetition_count`.
    pub fn decode_next(&mut self) -> Result<DecodedImageFrame, ImageError> {
        let frame = &self.frames[self.next_index];
        let mut decoder = WebPDecoder::new(frame.riff(&self.bytes)?)
            .map_err(|error| decode_error(error, frame.width, frame.height))?;
        if decoder.dimensions() != (frame.width, frame.height) {
            return Err(malformed());
        }
        decoder.set_memory_limit(MAX_DECODE_OUTPUT_BYTES as usize);
        let len = decoder.output_buffer_size().ok_or_else(malformed)?;
        let channels = if decoder.has_alpha() { 4 } else { 3 };
        let mut pixels = vec![0; len];
        decoder
            .read_image(&mut pixels)
            .map_err(|error| decode_error(error, frame.width, frame.height))?;

        let stride = self.width as usize * 4;
        self.canvas.resize(stride * self.height as usize, 0);
        if self.next_index == 0 {
            self.canvas.fill(0);
        } else {
            let previous = &self.frames[self.next_index - 1];
            if previous.dispose {
                for row in previous.y..previous.y + previous.height {
                    let start = row as usize * stride + previous.x as usize * 4;
                    self.canvas[start..start + previous.width as usize * 4].fill(0);
                }
            }
        }
        let frame_width = frame.width as usize;
        for (row, source) in pixels.chunks_exact(frame_width * channels).enumerate() {
            let start = (frame.y as usize + row) * stride + frame.x as usize * 4;
            let target = &mut self.canvas[start..start + frame_width * 4];
            match channels {
                3 => {
                    for (dst, src) in target.chunks_exact_mut(4).zip(source.chunks_exact(3)) {
                        dst[..3].copy_from_slice(src);
                        dst[3] = 255;
                    }
                }
                _ if !frame.blend => target.copy_from_slice(source),
                _ => {
                    for (dst, src) in target.chunks_exact_mut(4).zip(source.chunks_exact(4)) {
                        compose_over(dst, src);
                    }
                }
            }
        }
        self.next_index = (self.next_index + 1) % self.frame_count.get();
        Ok(DecodedImageFrame {
            image: ImageInfo::new(ImageHandle::from_rgba8(
                self.width,
                self.height,
                Arc::<[u8]>::from(self.canvas.as_slice()),
            )?),
            duration: frame.duration,
        })
    }
}

fn compose_over(dst: &mut [u8], src: &[u8]) {
    let a = u32::from(src[3]);
    if a == 0 {
        return;
    }
    if a == 255 || dst[3] == 0 {
        dst.copy_from_slice(src);
        return;
    }
    let source = a * 255;
    let under = u32::from(dst[3]) * (255 - a);
    let alpha = source + under;
    for (dst, src) in dst[..3].iter_mut().zip(&src[..3]) {
        *dst = ((u32::from(*src) * source + u32::from(*dst) * under + alpha / 2) / alpha) as u8;
    }
    dst[3] = ((alpha + 127) / 255) as u8;
}

impl FrameChunk {
    fn riff<'a>(&self, bytes: &'a [u8]) -> Result<FrameRiff<'a>, ImageError> {
        let data = &bytes[self.start..self.end];
        // A lossless VP8L subimage declares its own alpha. Lossy ALPH+VP8
        // needs an extended header because its separate alpha plane is
        // normally described by the outer container.
        let alpha = data.starts_with(b"ALPH");
        let prefix = if alpha { 30 } else { 12 };
        let riff_size = u32::try_from(data.len() + prefix - 8).map_err(|_| malformed())?;
        let mut header = [0; 30];
        header[..4].copy_from_slice(b"RIFF");
        header[4..8].copy_from_slice(&riff_size.to_le_bytes());
        header[8..12].copy_from_slice(b"WEBP");
        if alpha {
            header[12..16].copy_from_slice(b"VP8X");
            header[16..20].copy_from_slice(&10_u32.to_le_bytes());
            header[20] = 0x10;
            header[24..27].copy_from_slice(&(self.width - 1).to_le_bytes()[..3]);
            header[27..30].copy_from_slice(&(self.height - 1).to_le_bytes()[..3]);
        }
        Ok(FrameRiff {
            header,
            prefix,
            data,
            position: 0,
        })
    }
}

/// A small synthetic still-WebP header plus the original frame's borrowed
/// bytes. The codec can seek without copying the compressed subimage.
struct FrameRiff<'a> {
    header: [u8; 30],
    prefix: usize,
    data: &'a [u8],
    position: u64,
}

impl BufRead for FrameRiff<'_> {
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        if self.position < self.prefix as u64 {
            Ok(&self.header[self.position as usize..self.prefix])
        } else {
            let offset = self.position - self.prefix as u64;
            Ok(if offset < self.data.len() as u64 {
                &self.data[offset as usize..]
            } else {
                &[]
            })
        }
    }

    fn consume(&mut self, amount: usize) {
        self.position += amount as u64;
    }
}

impl Read for FrameRiff<'_> {
    fn read(&mut self, destination: &mut [u8]) -> io::Result<usize> {
        let source = self.fill_buf()?;
        let count = source.len().min(destination.len());
        destination[..count].copy_from_slice(&source[..count]);
        self.consume(count);
        Ok(count)
    }
}

impl Seek for FrameRiff<'_> {
    fn seek(&mut self, from: SeekFrom) -> io::Result<u64> {
        let end = self.prefix as u64 + self.data.len() as u64;
        let position = match from {
            SeekFrom::Start(position) => Some(position),
            SeekFrom::End(offset) => end.checked_add_signed(offset),
            SeekFrom::Current(offset) => self.position.checked_add_signed(offset),
        };
        self.position = position.ok_or_else(|| io::Error::from(io::ErrorKind::InvalidInput))?;
        Ok(self.position)
    }
}
