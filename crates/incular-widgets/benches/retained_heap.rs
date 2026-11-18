//! Requested heap bytes in a standalone process. Allocator metadata, native
//! allocations and GPU memory are outside this measurement.
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering::Relaxed};

use image::{ExtendedColorType, ImageEncoder, codecs::png::PngEncoder};
use incular_config::Constraints;
use incular_core::{Size, time::Instant};
use incular_image::ImageHandle;
use incular_widgets::internal::{Element, RenderKind, WidgetTree};
use incular_widgets::{Column, Directionality, Image, Row, Text, TextDirection, Widget};

struct CountingAllocator;
static LIVE: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);
static CALLS: AtomicUsize = AtomicUsize::new(0);

fn record_growth(bytes: usize) {
    let live = LIVE.fetch_add(bytes, Relaxed) + bytes;
    PEAK.fetch_max(live, Relaxed);
}

// SAFETY: all allocation operations are forwarded to System with the
// caller's layout. Accounting uses only non-allocating atomic operations.
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = unsafe { System.alloc(layout) };
        if !ptr.is_null() {
            CALLS.fetch_add(1, Relaxed);
            record_growth(layout.size());
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        LIVE.fetch_sub(layout.size(), Relaxed);
        unsafe { System.dealloc(ptr, layout) };
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let ptr = unsafe { System.realloc(ptr, layout, new_size) };
        if !ptr.is_null() {
            CALLS.fetch_add(1, Relaxed);
            if new_size >= layout.size() {
                record_growth(new_size - layout.size());
            } else {
                LIVE.fetch_sub(layout.size() - new_size, Relaxed);
            }
        }
        ptr
    }
}

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

fn main() {
    // The user opens a full-size attachment while the message list stays
    // mounted. Build the input assets outside all reported measurements.
    let encoded = [ExtendedColorType::Rgba8, ExtendedColorType::Rgb8].map(|format| {
        let channels = usize::from(format.channel_count());
        let pixels = vec![127; 1920 * 1080 * channels];
        let mut png = Vec::new();
        PngEncoder::new(&mut png)
            .write_image(&pixels, 1920, 1080, format)
            .unwrap();
        png
    });
    let start = LIVE.load(Relaxed);
    PEAK.store(start, Relaxed);
    let thumbnail = ImageHandle::from_rgba8(32, 32, vec![128; 32 * 32 * 4]).unwrap();
    let rows: Vec<Widget> = (0..2_500)
        .map(|index| {
            Row::new(vec![
                Widget::from(Image::new(thumbnail.clone())),
                Text::new(format!("Message {index}")).into(),
                Text::new("Read").into(),
            ])
            .into()
        })
        .collect();
    let mut tree = WidgetTree::new();
    tree.mount(Directionality::new(TextDirection::Ltr, Column::new(rows)).into())
        .unwrap();
    let mounted = LIVE.load(Relaxed) - start;
    let frame = |tree: &mut WidgetTree| {
        tree.layout(Constraints::tight(Size::new(640., 80_000.)))
            .unwrap();
        tree.update_compositor(Instant::now()).unwrap();
        std::hint::black_box(tree.paint());
    };
    let before_frame = CALLS.load(Relaxed);
    frame(&mut tree);
    let after_frame = CALLS.load(Relaxed);
    frame(&mut tree);
    let idle_calls = CALLS.load(Relaxed) - after_frame;
    let retained = LIVE.load(Relaxed) - start;
    let peak = PEAK.load(Relaxed) - start;
    let count = tree.element_count();
    println!(
        "elements={count} Widget={} Element={} RenderKind={}",
        size_of::<Widget>(),
        size_of::<Element>(),
        size_of::<RenderKind>()
    );
    println!(
        "mounted={mounted} retained={retained} peak={peak} bytes_per_element={:.1}",
        retained as f64 / count as f64
    );
    println!(
        "first_frame_allocations={} idle_frame_allocations={idle_calls}",
        after_frame - before_frame
    );
    for (name, png) in ["rgba", "rgb"].into_iter().zip(&encoded) {
        let before = LIVE.load(Relaxed);
        PEAK.store(before, Relaxed);
        let calls = CALLS.load(Relaxed);
        let preview = ImageHandle::from_bytes(png).unwrap();
        let extra = PEAK.load(Relaxed) - before;
        let calls = CALLS.load(Relaxed) - calls;
        println!(
            "{name}_decode_peak={extra} tree_plus_decode={} decode_allocations={calls}",
            retained + extra
        );
        std::hint::black_box(preview);
    }
    std::hint::black_box(tree);
}
