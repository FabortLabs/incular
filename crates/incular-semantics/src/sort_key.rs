use std::{any::Any, cmp::Ordering, fmt::{self, Debug}, sync::Arc};

use crate::OrdinalSortKey;

trait ErasedOrder: Debug + Send + Sync {
    fn as_any(&self) -> &dyn Any;
    fn compare(&self, other: &dyn ErasedOrder) -> Option<Ordering>;
}

impl<T: Ord + Debug + Send + Sync + 'static> ErasedOrder for T {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn compare(&self, other: &dyn ErasedOrder) -> Option<Ordering> {
        other
            .as_any()
            .downcast_ref::<T>()
            .map(|other| self.cmp(other))
    }
}

/// Orders a run of semantic siblings using a caller's Rust ordering type.
///
/// Separate types or group names are never compared, even when their values
/// look alike. Use a newtype to distinguish two orderings over the same
/// representation. Equal keys retain their original traversal order.
#[derive(Clone, Debug)]
pub struct CustomSemanticsSortKey {
    name: Option<Arc<str>>,
    value: Arc<dyn ErasedOrder>,
}

impl CustomSemanticsSortKey {
    #[must_use]
    pub fn new(value: impl Ord + Debug + Send + Sync + 'static) -> Self {
        Self {
            name: None,
            value: Arc::new(value),
        }
    }

    #[must_use]
    pub fn named(
        value: impl Ord + Debug + Send + Sync + 'static,
        name: impl Into<Arc<str>>,
    ) -> Self {
        Self {
            name: Some(name.into()),
            ..Self::new(value)
        }
    }

    #[must_use]
    pub fn is_compatible_with(&self, other: &Self) -> bool {
        self.name == other.name && self.value.as_any().type_id() == other.value.as_any().type_id()
    }

    /// Returns no ordering across different types or named groups.
    #[must_use]
    pub fn compare_to(&self, other: &Self) -> Option<Ordering> {
        if self.name != other.name {
            return None;
        }
        if Arc::ptr_eq(&self.value, &other.value) {
            return Some(Ordering::Equal);
        }
        self.value.compare(other.value.as_ref())
    }
}

impl PartialEq for CustomSemanticsSortKey {
    fn eq(&self, other: &Self) -> bool {
        self.compare_to(other) == Some(Ordering::Equal)
    }
}

impl Eq for CustomSemanticsSortKey {}


/// A borrowed, dynamically typed Flutter-style semantics sort key.
#[derive(Clone, Copy, Debug)]
pub enum SemanticsSortKeyRef<'a> {
    Ordinal(&'a OrdinalSortKey),
    Custom(&'a CustomSemanticsSortKey),
}

/// Flutter only compares semantic sort keys of the same concrete type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IncompatibleSemanticsSortKeyTypes;

impl fmt::Display for IncompatibleSemanticsSortKeyTypes {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("semantics sort keys must have the same concrete type")
    }
}

impl std::error::Error for IncompatibleSemanticsSortKeyTypes {}

impl SemanticsSortKeyRef<'_> {
    /// Compares names before values, but only after checking the type.
    ///
    /// # Errors
    /// Returns an error when the key types are incompatible.
    pub fn compare_to(self, other: Self) -> Result<Ordering, IncompatibleSemanticsSortKeyTypes> {
        match (self, other) {
            (Self::Ordinal(left), Self::Ordinal(right)) => Ok(left.compare_to(right)),
            (Self::Custom(left), Self::Custom(right))
                if left.value.as_any().type_id() == right.value.as_any().type_id() =>
            {
                let names = compare_names(left.name.as_deref(), right.name.as_deref());
                if names != Ordering::Equal {
                    return Ok(names);
                }
                left.compare_to(right).ok_or(IncompatibleSemanticsSortKeyTypes)
            }
            _ => Err(IncompatibleSemanticsSortKeyTypes),
        }
    }

    /// Whether the retained traversal sorter can place both in one run.
    #[must_use]
    pub fn shares_group_with(self, other: Self) -> bool {
        match (self, other) {
            (Self::Ordinal(left), Self::Ordinal(right)) => left.name == right.name,
            (Self::Custom(left), Self::Custom(right)) => left.is_compatible_with(right),
            _ => false,
        }
    }
}

pub(super) fn compare_names(left: Option<&str>, right: Option<&str>) -> Ordering {
    match (left, right) {
        (None, Some(_)) => Ordering::Less,
        (Some(_), None) => Ordering::Greater,
        (Some(left), Some(right)) if left != right => {
            left.encode_utf16().cmp(right.encode_utf16())
        }
        _ => Ordering::Equal,
    }
}
