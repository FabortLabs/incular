//! Layout pools are bounded by approximate bytes and shrink after a burst.
//! Run with --nocapture for the measured numbers.

use incular_text::{TextAlign, TextEngine, TextStyle};
use std::alloc::{GlobalAlloc, Layout, System};
use std::hint::black_box;
use std::sync::Mutex;
use std::sync::atomic::{AtomicIsize, Ordering::Relaxed};

struct CountingAllocator;
static LIVE: AtomicIsize = AtomicIsize::new(0);
/// Tests share the global counter, so they measure one at a time.
static SERIAL: Mutex<()> = Mutex::new(());

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let pointer = unsafe { System.alloc(layout) };
        if !pointer.is_null() {
            LIVE.fetch_add(layout.size() as isize, Relaxed);
        }
        pointer
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let pointer = unsafe { System.alloc_zeroed(layout) };
        if !pointer.is_null() {
            LIVE.fetch_add(layout.size() as isize, Relaxed);
        }
        pointer
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        unsafe { System.dealloc(pointer, layout) };
        LIVE.fetch_sub(layout.size() as isize, Relaxed);
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let result = unsafe { System.realloc(pointer, layout, new_size) };
        if !result.is_null() {
            LIVE.fetch_add(new_size as isize - layout.size() as isize, Relaxed);
        }
        result
    }
}

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

fn serial() -> std::sync::MutexGuard<'static, ()> {
    SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn mib(bytes: isize) -> f64 {
    bytes as f64 / f64::from(1 << 20)
}

#[test]
fn pools_shrink_back_after_a_burst_of_distinct_labels() {
    let _serial = serial();
    let style = TextStyle::default();
    let mut engine = TextEngine::new();
    let recurring: Vec<String> = (0..100).map(|index| format!("recurring {index}")).collect();
    let burst: Vec<String> = (0..20_000)
        .map(|index| format!("burst label {index}"))
        .collect();
    for label in &recurring {
        black_box(engine.layout(label, &style, None, TextAlign::Start));
    }
    let baseline = LIVE.load(Relaxed);

    // Cycles that outgrow the pool miss keys it just evicted, which proves
    // thrashing; growing cycles grow the pools step by step to 32768.
    for size in [3000, 6000, 12_000, 20_000] {
        for _ in 0..3 {
            for label in &burst[..size] {
                black_box(engine.layout(label, &style, None, TextAlign::Start));
            }
        }
    }
    let peak = LIVE.load(Relaxed);
    assert!(engine.diagnostics().cache_growths > 0);

    // A frame-like working set of 100 labels, laid out 50 times.
    for _ in 0..50 {
        for label in &recurring {
            black_box(engine.layout(label, &style, None, TextAlign::Start));
        }
    }
    let settled = LIVE.load(Relaxed);
    assert!(engine.diagnostics().cache_shrinks > 0, "pools never shrank");

    // The working set stays warm through the shrink.
    let misses = engine.diagnostics().cache_misses;
    for label in &recurring {
        black_box(engine.layout(label, &style, None, TextAlign::Start));
    }
    assert_eq!(engine.diagnostics().cache_misses, misses);

    println!(
        "BURST baseline=0 peak={:.2}MiB settled={:.2}MiB accounted={:.2}MiB",
        mib(peak - baseline),
        mib(settled - baseline),
        mib(engine.cache_bytes() as isize),
    );
    assert!(
        (settled - baseline) * 5 < peak - baseline,
        "retained {} of a {}-byte burst",
        settled - baseline,
        peak - baseline,
    );
}

#[test]
fn byte_budget_bounds_retained_layouts() {
    let _serial = serial();
    let style = TextStyle::default();
    let mut engine = TextEngine::new();
    let budget = 1 << 20;
    engine.set_cache_byte_budget(budget);
    let _ = engine.layout("font warmup", &style, None, TextAlign::Start);
    let baseline = LIVE.load(Relaxed);
    for index in 0..20_000 {
        let label = format!("{index:05} a label long enough to carry a few dozen glyphs");
        black_box(engine.layout(&label, &style, None, TextAlign::Start));
    }
    let retained = LIVE.load(Relaxed) - baseline;
    println!(
        "BUDGET budget={:.2}MiB retained={:.2}MiB accounted={:.2}MiB",
        mib(budget as isize),
        mib(retained),
        mib(engine.cache_bytes() as isize),
    );
    assert!(engine.cache_bytes() <= budget);
    // The estimate counts layouts shared between pools twice, and the
    // pools' hash tables keep their peak size until they shrink.
    assert!(retained < budget as isize * 2, "retained {retained} bytes");
}

#[test]
fn wrapped_paragraph_estimate_tracks_allocation() {
    let _serial = serial();
    let style = TextStyle::default();
    let mut engine = TextEngine::new();
    let _ = engine.layout("font warmup", &style, None, TextAlign::Start);
    let paragraphs: Vec<String> = (0..200)
        .map(|index| format!("{index} {}", "wrapping paragraph text ".repeat(12)))
        .collect();
    let baseline = LIVE.load(Relaxed);
    let accounted_before = engine.cache_bytes();
    for paragraph in &paragraphs {
        black_box(engine.layout(paragraph, &style, Some(120.), TextAlign::Start));
    }
    let retained = LIVE.load(Relaxed) - baseline;
    let accounted = engine.cache_bytes() - accounted_before;
    println!(
        "WRAPPED retained={:.2}MiB accounted={:.2}MiB",
        mib(retained),
        mib(accounted as isize),
    );
    // The estimate is within a factor of two of the real footprint.
    assert!(accounted as isize * 2 > retained && (accounted as isize) < retained * 2);
}

#[test]
fn cyclic_working_set_stays_warm_while_decay_runs() {
    let _serial = serial();
    let style = TextStyle::default();
    let mut engine = TextEngine::new();
    let labels: Vec<String> = (0..3000).map(|index| format!("label {index}")).collect();
    let mut misses = Vec::new();
    for _ in 0..8 {
        let before = engine.diagnostics().cache_misses;
        for label in &labels {
            engine.layout(label, &style, Some(400.), TextAlign::Center);
        }
        misses.push(engine.diagnostics().cache_misses - before);
    }
    assert!(
        misses[2..].iter().all(|&count| count == 0),
        "decay evicted a cycle that fits: {misses:?}"
    );
}

#[test]
fn byte_budget_keeps_a_long_documents_paragraphs() {
    let _serial = serial();
    let style = TextStyle::default();
    let mut engine = TextEngine::new();
    engine.set_cache_byte_budget(64 << 10);
    let body: String = (0..3000)
        .map(|index| format!("let line_{index} = {index};\n"))
        .collect();
    black_box(engine.layout_with_options(
        &format!("// edit 0\n{body}"),
        &style,
        Default::default(),
    ));
    let before = engine.diagnostics().paragraphs_reshaped;
    // Editing the first line looks every later paragraph up again.
    for edit in 1..=20 {
        black_box(engine.layout_with_options(
            &format!("// edit {edit}\n{body}"),
            &style,
            Default::default(),
        ));
    }
    let reshaped = engine.diagnostics().paragraphs_reshaped - before;
    assert_eq!(reshaped, 20, "each edit reshapes only the edited line");
}
