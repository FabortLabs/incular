//! Memory behaviour of undo history on a large document.
//! Run alone with --nocapture; the printed number is diagnostic evidence.

use incular_runtime::UndoHistoryController;
use incular_text::{TextEditingController, TextEditingValue, TextSelection};
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicIsize, Ordering::Relaxed};

struct CountingAllocator;
static LIVE: AtomicIsize = AtomicIsize::new(0);

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

#[test]
fn large_document_history_retains_only_deltas() {
    let original: String = (0..5_000)
        .map(|line| format!("{line:05} the quick brown fox jumps over the lazy dog, again\n"))
        .collect();
    assert!(original.len() > 250_000);
    let editor = TextEditingController::with_text(original.clone());
    let controller = UndoHistoryController::new(editor.clone());
    let middle = original.len() / 2;
    let line_start = original[..middle].rfind('\n').unwrap() + 1;

    let before = LIVE.load(Relaxed);
    for step in 0..100 {
        let mut value = editor.value();
        let at = line_start + step;
        value.text.insert(at, 'x');
        value.selection = TextSelection::collapsed(at + 1);
        editor.set_value(value);
    }
    let growth = LIVE.load(Relaxed) - before;
    println!("UNDO_MEMORY commits=100 live_growth_bytes={growth}");
    assert!(growth < 1_000_000, "history retained {growth} bytes");

    let final_text = editor.text();
    assert_eq!(final_text.len(), original.len() + 100);
    for _ in 0..100 {
        assert!(controller.undo());
    }
    assert!(!controller.can_undo());
    assert_eq!(editor.text(), original);
    for _ in 0..100 {
        assert!(controller.redo());
    }
    assert!(!controller.can_redo());
    assert_eq!(editor.text(), final_text);
}

#[test]
fn mixed_edits_round_trip_exact_values() {
    let editor = TextEditingController::with_text("héllo wörld");
    let controller = UndoHistoryController::new(editor.clone());
    let mut recorded = vec![editor.value()];
    let edits: Vec<(&str, usize)> = vec![
        ("héllo 😀 wörld", "héllo 😀".len()),
        ("héllo 😀 wörld!", "héllo 😀 wörld!".len()),
        ("héllo 😀 wörld", "héllo 😀 wörld".len()),
        ("hello 😀 wörld", 5),
        ("hello 😁 wörld", "hello 😁".len()),
        ("hello 😁 wörld wörld", 0),
        ("é", 2),
        ("", 0),
        ("éé", 4),
        ("éèé", 4),
    ];
    for (text, caret) in edits {
        let value = TextEditingValue {
            text: text.to_owned(),
            selection: TextSelection::collapsed(caret),
            composing: None,
        };
        editor.set_value(value);
        recorded.push(editor.value());
        // A pure selection change does not create a step but is tracked.
        let last = recorded.last_mut().unwrap();
        if last.text.is_char_boundary(0) {
            let mut moved = last.clone();
            moved.selection = TextSelection::new(0, caret);
            editor.set_value(moved);
            *last = editor.value();
        }
    }
    for expected in recorded.iter().rev().skip(1) {
        assert!(controller.undo());
        let value = editor.value();
        assert_eq!(value.text, expected.text);
    }
    assert!(!controller.undo());
    for expected in recorded.iter().skip(1) {
        assert!(controller.redo());
        let value = editor.value();
        assert_eq!(value.text, expected.text);
    }
    assert!(!controller.redo());
}

#[test]
fn undo_restores_recorded_selection() {
    let editor = TextEditingController::with_text("a😀b");
    let controller = UndoHistoryController::new(editor.clone());
    let first = editor.value();
    let mut next = first.clone();
    next.text = "a😀bé".to_owned();
    next.selection = TextSelection::new(1, 5);
    editor.set_value(next.clone());
    let mut third = next.clone();
    third.text = "aé".to_owned();
    third.selection = TextSelection::collapsed(1);
    editor.set_value(third.clone());
    assert!(controller.undo());
    assert_eq!(editor.value(), next);
    assert!(controller.undo());
    assert_eq!(editor.value(), first);
    assert!(controller.redo());
    assert_eq!(editor.value(), next);
    assert!(controller.redo());
    assert_eq!(editor.value(), third);
}
