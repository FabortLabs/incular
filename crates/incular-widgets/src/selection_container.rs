//! Flutter-shaped selection container widget descriptors.

use std::{cell::RefCell, collections::HashSet, rc::Rc};

use crate::{SelectionAreaController, SelectionContainerDelegate, Widget, tree::ElementId};

/// A shared registry for selectable containers hosted outside a selection area.
/// Clone the handle to observe registration without owning the widget tree.
#[derive(Clone, Debug, Default)]
pub struct SelectionRegistrar(Rc<RefCell<HashSet<(u64, ElementId)>>>);

impl PartialEq for SelectionRegistrar {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl SelectionRegistrar {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn registered_child_count(&self) -> usize {
        self.0.borrow().len()
    }

    pub(crate) fn register(self, tree: u64, id: ElementId) -> SelectionRegistration {
        let member = (tree, id);
        let fresh = self.0.borrow_mut().insert(member);
        debug_assert!(fresh);
        SelectionRegistration {
            registrar: self,
            member,
        }
    }
}

pub(crate) struct SelectionRegistration {
    pub(crate) registrar: SelectionRegistrar,
    member: (u64, ElementId),
}

impl Drop for SelectionRegistration {
    fn drop(&mut self) {
        self.registrar.0.borrow_mut().remove(&self.member);
    }
}

/// Provides a registrar to descendant selection containers without an
/// explicit registrar, up to the closest nested selection boundary.
#[derive(Clone, Debug, PartialEq)]
pub struct SelectionRegistrarScope {
    registrar: SelectionRegistrar,
    child: Widget,
}

impl SelectionRegistrarScope {
    #[must_use]
    pub fn new(registrar: SelectionRegistrar, child: impl Into<Widget>) -> Self {
        Self {
            registrar,
            child: child.into(),
        }
    }
}

impl From<SelectionRegistrarScope> for Widget {
    fn from(value: SelectionRegistrarScope) -> Self {
        Widget::environment_scope(value.registrar, value.child)
    }
}

/// A retained selection boundary around a widget subtree.
///
/// The boundary owns a delegate/controller pair. A non-empty enabled
/// container counts as one entry in its enclosing selection registrar. Use [`SelectionContainer::disabled`] to keep a
/// subtree visible while preventing its selectable children from registering.
#[derive(Clone, Debug, PartialEq)]
pub struct SelectionContainer {
    child: Widget,
    delegate: SelectionContainerDelegate,
    registrar: Option<SelectionRegistrar>,
}

impl SelectionContainer {
    /// Creates an enabled container with a fresh selection registry.
    #[must_use]
    pub fn new(child: impl Into<Widget>) -> Self {
        Self::with_delegate(SelectionContainerDelegate::new(), child)
    }

    /// Creates a container backed by an explicit retained delegate.
    #[must_use]
    pub fn with_delegate(delegate: SelectionContainerDelegate, child: impl Into<Widget>) -> Self {
        Self {
            child: child.into(),
            delegate,
            registrar: None,
        }
    }

    /// Creates an enabled container backed by an explicit controller.
    #[must_use]
    pub fn with_controller(controller: SelectionAreaController, child: impl Into<Widget>) -> Self {
        Self::with_delegate(
            SelectionContainerDelegate::with_controller(controller),
            child,
        )
    }

    /// Creates a container that deliberately does not register selectable
    /// descendants. This mirrors Flutter's disabled selection boundary.
    #[must_use]
    pub fn disabled(child: impl Into<Widget>) -> Self {
        Self::with_delegate(SelectionContainerDelegate::disabled(), child)
    }

    /// Uses a supplied registrar rather than the enclosing registrar.
    #[must_use]
    pub fn registrar(mut self, registrar: SelectionRegistrar) -> Self {
        self.registrar = Some(registrar);
        self
    }

    #[must_use]
    pub fn delegate(&self) -> &SelectionContainerDelegate {
        &self.delegate
    }

    #[must_use]
    pub fn controller(&self) -> SelectionAreaController {
        self.delegate.controller()
    }

    #[must_use]
    pub fn child(&self) -> &Widget {
        &self.child
    }
}

impl From<SelectionContainer> for Widget {
    fn from(value: SelectionContainer) -> Self {
        Widget::selection_container_in(value.delegate, value.registrar, value.child)
    }
}
