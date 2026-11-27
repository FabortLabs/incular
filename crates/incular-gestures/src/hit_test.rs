//! Paint-to-hit coordinate conversion and a retained hit path.

use std::{
    cell::{Ref, RefCell},
    rc::Rc,
};

use incular_core::{Offset, Transform};

/// How a parent's hit coordinate is mapped into a child.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum HitTestTransform {
    /// Where the child was placed at paint time.
    PaintOffset(Offset),
    /// A child-to-parent transform. A singular transform cannot hit.
    Paint(Transform),
    /// An already parent-to-child transform.
    Raw(Transform),
}

impl HitTestTransform {
    #[must_use]
    pub fn resolve(self) -> Option<Transform> {
        match self {
            Self::PaintOffset(offset) => Some(Transform::translation(Offset::ZERO - offset)),
            Self::Paint(transform) => transform.inverse(),
            Self::Raw(transform) => Some(transform),
        }
    }
}

/// One target in hit order, together with its global-to-local transform.
#[derive(Clone, Debug, PartialEq)]
pub struct HitTestEntry<T> {
    pub target: T,
    pub transform: Transform,
}

/// Owns a hit path and its transform stack.
#[derive(Clone, Debug)]
pub struct HitTestResult<T> {
    path: Vec<HitTestEntry<T>>,
    transforms: Vec<Transform>,
}

fn compose(parent: Option<Transform>, local: Transform) -> Transform {
    parent.map_or(local, |parent| local.then(parent))
}

impl<T> Default for HitTestResult<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> HitTestResult<T> {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            path: Vec::new(),
            transforms: Vec::new(),
        }
    }

    #[must_use]
    pub fn path(&self) -> &[HitTestEntry<T>] {
        &self.path
    }

    pub fn add(&mut self, target: T) {
        self.path.push(HitTestEntry {
            target,
            transform: self
                .transforms
                .last()
                .copied()
                .unwrap_or(Transform::IDENTITY),
        });
    }

    pub fn push_transform(&mut self, transform: Transform) {
        self.transforms
            .push(compose(self.transforms.last().copied(), transform));
    }

    /// Pushes a caller-to-child translation (not the inverse paint offset).
    pub fn push_offset(&mut self, offset: Offset) {
        self.push_transform(Transform::translation(offset));
    }

    pub fn pop_transform(&mut self) -> Option<Transform> {
        self.transforms.pop()
    }

    /// Shares this transaction with independently retained result handles.
    /// Ordinary owned hit tests need no reference counting or interior borrows.
    #[must_use]
    pub fn into_shared(self) -> SharedHitTestResult<T> {
        SharedHitTestResult(Rc::new(RefCell::new(self)))
    }
}

/// A live view of one hit-test transaction. Wrapping shares both the path
/// and the active transform stack, not a snapshot of either.
#[derive(Debug)]
pub struct SharedHitTestResult<T>(Rc<RefCell<HitTestResult<T>>>);

impl<T> Clone for SharedHitTestResult<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T> SharedHitTestResult<T> {
    #[must_use]
    pub fn wrap(result: &Self) -> Self {
        result.clone()
    }

    /// Another wrapper can observe appended entries. Drop a path borrow
    /// before changing the shared transaction.
    #[must_use]
    pub fn path(&self) -> Ref<'_, [HitTestEntry<T>]> {
        Ref::map(self.0.borrow(), HitTestResult::path)
    }

    #[must_use]
    pub fn shares_path_with(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }

    pub fn add(&self, target: T) {
        self.0.borrow_mut().add(target);
    }

    pub fn push_transform(&self, transform: Transform) {
        self.0.borrow_mut().push_transform(transform);
    }

    pub fn push_offset(&self, offset: Offset) {
        self.0.borrow_mut().push_offset(offset);
    }

    pub fn pop_transform(&self) -> Option<Transform> {
        self.0.borrow_mut().pop_transform()
    }
}

/// A box hit-test view over the same path. Nested tests borrow the path
/// and scope the transform to that branch without retaining another stack.
pub struct BoxHitTestResult<'a, T> {
    path: &'a mut Vec<HitTestEntry<T>>,
    transform: Option<Transform>,
}

impl<'a, T> BoxHitTestResult<'a, T> {
    #[must_use]
    pub fn wrap(result: &'a mut HitTestResult<T>) -> Self {
        Self {
            path: &mut result.path,
            transform: result.transforms.last().copied(),
        }
    }

    #[must_use]
    pub fn path(&self) -> &[HitTestEntry<T>] {
        self.path
    }

    pub fn add(&mut self, target: T) {
        self.path.push(HitTestEntry {
            target,
            transform: self.transform.unwrap_or(Transform::IDENTITY),
        });
    }

    pub fn add_with_raw_transform(
        &mut self,
        transform: Option<Transform>,
        position: Offset,
        hit_test: impl FnOnce(&mut BoxHitTestResult<'_, T>, Offset) -> bool,
    ) -> bool {
        let mut view = BoxHitTestResult {
            path: &mut *self.path,
            transform: transform
                .map_or(self.transform, |local| Some(compose(self.transform, local))),
        };
        let position = transform.map_or(position, |raw| raw.transform_point(position));
        hit_test(&mut view, position)
    }

    pub fn add_with_paint_transform(
        &mut self,
        transform: Option<Transform>,
        position: Offset,
        hit_test: impl FnOnce(&mut BoxHitTestResult<'_, T>, Offset) -> bool,
    ) -> bool {
        match transform {
            Some(paint) => HitTestTransform::Paint(paint)
                .resolve()
                .is_some_and(|raw| self.add_with_raw_transform(Some(raw), position, hit_test)),
            None => self.add_with_raw_transform(None, position, hit_test),
        }
    }

    pub fn add_with_paint_offset(
        &mut self,
        offset: Option<Offset>,
        position: Offset,
        hit_test: impl FnOnce(&mut BoxHitTestResult<'_, T>, Offset) -> bool,
    ) -> bool {
        self.add_with_raw_transform(
            offset.and_then(|offset| HitTestTransform::PaintOffset(offset).resolve()),
            position,
            hit_test,
        )
    }

    /// Uses a transform for path entries when the caller does its own geometry
    /// and does not have a position to hand to the delegate.
    pub fn add_with_out_of_band_position(
        &mut self,
        transform: HitTestTransform,
        hit_test: impl FnOnce(&mut BoxHitTestResult<'_, T>) -> bool,
    ) -> bool {
        let resolved = transform.resolve();
        debug_assert!(resolved.is_some(), "paintTransform must be invertible.");
        resolved.is_some_and(|transform| {
            let mut view = BoxHitTestResult {
                path: &mut *self.path,
                transform: Some(compose(self.transform, transform)),
            };
            hit_test(&mut view)
        })
    }
}
