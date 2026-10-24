//! Flutter's `SliverChildDelegate`s (`widgets/scroll_delegate.dart`): how a
//! lazy sliver gets its children and what each child is wrapped in.

use std::rc::Rc;

use crate::{AutomaticKeepAlive, IndexedSemantics, RepaintBoundary, Widget};

/// Maps a child and its index to its semantic index, or `None` to leave it
/// out of the semantic indexing (Flutter's `SemanticIndexCallback`).
pub type SemanticIndexCallback = Rc<dyn Fn(&Widget, usize) -> Option<usize>>;

/// Flutter's `SliverChildDelegate`: the children of a lazy sliver.
pub trait SliverChildDelegate {
    /// The child at `index`, wrapped as the delegate's flags say, or `None`
    /// past the last child.
    fn build(&self, index: usize) -> Option<Widget>;

    /// Flutter's `estimatedChildCount`: the number of children, which lazy
    /// slivers lay out exactly.
    fn estimated_child_count(&self) -> usize;
}

/// The wrapping flags shared by Flutter's builder and list delegates.
#[derive(Clone)]
struct ChildWrapping {
    add_automatic_keep_alives: bool,
    add_repaint_boundaries: bool,
    add_semantic_indexes: bool,
    semantic_index_offset: usize,
    semantic_index_callback: SemanticIndexCallback,
}

impl Default for ChildWrapping {
    fn default() -> Self {
        Self {
            add_automatic_keep_alives: true,
            add_repaint_boundaries: true,
            add_semantic_indexes: true,
            semantic_index_offset: 0,
            // `_kDefaultSemanticIndexCallback`: the local index.
            semantic_index_callback: Rc::new(|_, index| Some(index)),
        }
    }
}

impl ChildWrapping {
    /// `SliverChildBuilderDelegate.build`'s wrapping, innermost first:
    /// repaint boundary, semantic index, automatic keep-alive; the child's
    /// key stays on the outermost widget.
    fn wrap(&self, child: Widget, index: usize) -> Widget {
        let key = child.key().cloned();
        let mut child = child;
        if self.add_repaint_boundaries {
            child = RepaintBoundary::new(child).into();
        }
        if self.add_semantic_indexes
            && let Some(semantic_index) = (self.semantic_index_callback)(&child, index)
        {
            child =
                IndexedSemantics::new(semantic_index + self.semantic_index_offset, child).into();
        }
        if self.add_automatic_keep_alives {
            child = AutomaticKeepAlive::new(child).into();
        }
        match key {
            Some(key) => child.with_key(key),
            None => child,
        }
    }
}

macro_rules! wrapping_setters {
    () => {
        /// Whether each child is wrapped in an `AutomaticKeepAlive`.
        /// Defaults to true.
        #[must_use]
        pub fn add_automatic_keep_alives(mut self, add: bool) -> Self {
            self.wrapping.add_automatic_keep_alives = add;
            self
        }

        /// Whether each child is wrapped in a `RepaintBoundary`. Defaults to
        /// true.
        #[must_use]
        pub fn add_repaint_boundaries(mut self, add: bool) -> Self {
            self.wrapping.add_repaint_boundaries = add;
            self
        }

        /// Whether each child is wrapped in an `IndexedSemantics`. Defaults
        /// to true.
        #[must_use]
        pub fn add_semantic_indexes(mut self, add: bool) -> Self {
            self.wrapping.add_semantic_indexes = add;
            self
        }

        /// Added to every semantic index. Defaults to 0.
        #[must_use]
        pub fn semantic_index_offset(mut self, offset: usize) -> Self {
            self.wrapping.semantic_index_offset = offset;
            self
        }

        /// Defaults to the child's index.
        #[must_use]
        pub fn semantic_index_callback(
            mut self,
            callback: impl Fn(&Widget, usize) -> Option<usize> + 'static,
        ) -> Self {
            self.wrapping.semantic_index_callback = Rc::new(callback);
            self
        }
    };
}

/// Flutter's `SliverChildBuilderDelegate`: children built on demand.
#[derive(Clone)]
pub struct SliverChildBuilderDelegate {
    builder: Rc<dyn Fn(usize) -> Option<Widget>>,
    child_count: Option<usize>,
    wrapping: ChildWrapping,
}

impl SliverChildBuilderDelegate {
    /// Without a [`Self::child_count`], the children end at the first index
    /// for which `builder` returns `None`.
    #[must_use]
    pub fn new(builder: impl Fn(usize) -> Option<Widget> + 'static) -> Self {
        Self {
            builder: Rc::new(builder),
            child_count: None,
            wrapping: ChildWrapping::default(),
        }
    }

    #[must_use]
    pub fn child_count(mut self, count: usize) -> Self {
        self.child_count = Some(count);
        self
    }

    wrapping_setters!();
}

impl SliverChildDelegate for SliverChildBuilderDelegate {
    fn build(&self, index: usize) -> Option<Widget> {
        if self.child_count.is_some_and(|count| index >= count) {
            return None;
        }
        (self.builder)(index).map(|child| self.wrapping.wrap(child, index))
    }

    fn estimated_child_count(&self) -> usize {
        self.child_count.unwrap_or_else(|| {
            (0..)
                .take_while(|index| (self.builder)(*index).is_some())
                .count()
        })
    }
}

/// Flutter's `SliverChildListDelegate`: an explicit list of children.
#[derive(Clone)]
pub struct SliverChildListDelegate {
    children: Rc<[Widget]>,
    wrapping: ChildWrapping,
}

impl SliverChildListDelegate {
    #[must_use]
    pub fn new(children: impl IntoIterator<Item = Widget>) -> Self {
        Self {
            children: children.into_iter().collect(),
            wrapping: ChildWrapping::default(),
        }
    }

    wrapping_setters!();
}

impl SliverChildDelegate for SliverChildListDelegate {
    fn build(&self, index: usize) -> Option<Widget> {
        self.children
            .get(index)
            .map(|child| self.wrapping.wrap(child.clone(), index))
    }

    fn estimated_child_count(&self) -> usize {
        self.children.len()
    }
}

/// A delegate as the count and index builder the lazy slivers take.
pub(super) fn delegate_builder(
    delegate: Rc<dyn SliverChildDelegate>,
) -> (usize, impl Fn(usize) -> Widget + 'static) {
    let count = delegate.estimated_child_count();
    (count, move |index| {
        delegate.build(index).unwrap_or_else(|| {
            unreachable!("lazy slivers build only indexes below the delegate's child count")
        })
    })
}
