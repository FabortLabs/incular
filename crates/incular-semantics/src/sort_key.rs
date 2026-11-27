use std::{any::Any, cmp::Ordering, fmt::Debug, sync::Arc};

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
