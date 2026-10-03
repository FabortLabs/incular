//! Small crate-private helpers shared by unrelated widget modules.

use std::rc::Rc;

/// Identity equality for optional shared callbacks. Closures have no
/// structural equality, so two configurations match only when they share one
/// allocation or both are absent.
pub(crate) fn same_rc<T: ?Sized>(left: &Option<Rc<T>>, right: &Option<Rc<T>>) -> bool {
    match (left, right) {
        (Some(left), Some(right)) => Rc::ptr_eq(left, right),
        (None, None) => true,
        _ => false,
    }
}

/// Truncates `text` to at most `limit` characters, marking elision with `…`.
pub(crate) fn truncate(text: &str, limit: usize) -> String {
    match text.char_indices().nth(limit) {
        Some((end, _)) => format!("{}…", &text[..end]),
        None => text.to_owned(),
    }
}
