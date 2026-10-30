//! Flutter's `RawMenuAnchor`, `RawMenuAnchorGroup` and `MenuController`
//! (`widgets/raw_menu_anchor.dart`): unstyled menus whose overlays the
//! application builds from the anchor's geometry.

use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
    sync::atomic::{AtomicU64, Ordering},
};

use incular_core::{Offset, Rect, Size};

use crate::tree::RetainedState;
use crate::{
    BuildContext, FocusNode, OverlayChildLayoutInfo, OverlayPortal, TapRegion, TapRegionGroupId,
    TransientDismissPolicy, TransientDismissReason, TransientRole, Widget,
};

/// Flutter's `RawMenuOverlayInfo`: what a [`RawMenuAnchor`] overlay
/// builder receives.
#[derive(Clone, Debug, PartialEq)]
pub struct RawMenuOverlayInfo {
    /// The anchor's bounds in the overlay.
    pub anchor_rect: Rect,
    pub overlay_size: Size,
    /// The position [`MenuController::open`] was given, relative to the
    /// anchor.
    pub position: Option<Offset>,
    /// The tap region group the menu tree shares; overlay content wrapped
    /// in a [`TapRegion`] of this group counts as inside the menu.
    pub tap_region_group_id: TapRegionGroupId,
}

type Callback = Rc<dyn Fn()>;
/// `RawMenuAnchor.onOpenRequested`: receives the position and a callback
/// that shows the overlay.
pub type MenuOpenRequested = Rc<dyn Fn(Option<Offset>, Rc<dyn Fn()>)>;
/// `RawMenuAnchor.onCloseRequested`: receives a callback that hides the
/// overlay.
pub type MenuCloseRequested = Rc<dyn Fn(Rc<dyn Fn()>)>;

#[derive(Default)]
struct Callbacks {
    on_open: Option<Callback>,
    on_close: Option<Callback>,
    on_open_requested: Option<MenuOpenRequested>,
    on_close_requested: Option<MenuCloseRequested>,
}

/// One anchor or group in a menu tree.
struct AnchorNode {
    is_group: bool,
    open: Cell<bool>,
    position: Cell<Option<Offset>>,
    parent: RefCell<Weak<AnchorNode>>,
    children: RefCell<Vec<Weak<AnchorNode>>>,
    callbacks: RefCell<Callbacks>,
    /// `childFocusNode`: focused when a root overlay anchor opens.
    child_focus_node: RefCell<Option<FocusNode>>,
    /// The view size a root saw at its last build; a change closes it.
    view_size: Cell<Option<Size>>,
    group_id: u64,
    revision: RefCell<Rc<Cell<u64>>>,
}

impl AnchorNode {
    fn new(is_group: bool, revision: Rc<Cell<u64>>) -> Rc<Self> {
        static NEXT: AtomicU64 = AtomicU64::new(1);
        Rc::new(Self {
            is_group,
            open: Cell::new(false),
            position: Cell::new(None),
            parent: RefCell::new(Weak::new()),
            children: RefCell::new(Vec::new()),
            callbacks: RefCell::new(Callbacks::default()),
            child_focus_node: RefCell::new(None),
            view_size: Cell::new(None),
            group_id: NEXT.fetch_add(1, Ordering::Relaxed),
            revision: RefCell::new(revision),
        })
    }

    fn rebuild(&self) {
        let revision = self.revision.borrow().clone();
        revision.set(revision.get().wrapping_add(1));
    }

    fn parent(&self) -> Option<Rc<Self>> {
        self.parent.borrow().upgrade()
    }

    fn root(self: &Rc<Self>) -> Rc<Self> {
        let mut node = self.clone();
        while let Some(parent) = node.parent() {
            node = parent;
        }
        node
    }

    fn live_children(&self) -> Vec<Rc<Self>> {
        let mut children = self.children.borrow_mut();
        children.retain(|child| child.strong_count() > 0);
        children.iter().filter_map(Weak::upgrade).collect()
    }

    /// A group is open while any of its menus is.
    fn is_open(&self) -> bool {
        if self.is_group {
            self.live_children().iter().any(|child| child.is_open())
        } else {
            self.open.get()
        }
    }

    fn attach(self: &Rc<Self>, parent: Option<Rc<Self>>) {
        let current = self.parent();
        if current.as_ref().map(Rc::as_ptr) == parent.as_ref().map(Rc::as_ptr) {
            return;
        }
        if let Some(current) = current {
            current
                .children
                .borrow_mut()
                .retain(|child| !std::ptr::eq(child.as_ptr(), Rc::as_ptr(self)));
        }
        if let Some(parent) = &parent {
            parent.children.borrow_mut().push(Rc::downgrade(self));
        }
        *self.parent.borrow_mut() = parent.as_ref().map_or_else(Weak::new, Rc::downgrade);
    }

    /// `closeChildren`: asks each submenu to close, through its
    /// `onCloseRequested`.
    fn close_children(&self) {
        for child in self.live_children() {
            child.request_close();
        }
    }

    /// `_childChangedOpenState`: the ancestors' open state may have
    /// changed with this menu's.
    fn child_changed_open_state(&self) {
        let mut ancestor = self.parent();
        while let Some(node) = ancestor {
            node.rebuild();
            ancestor = node.parent();
        }
    }

    /// `_isRootOverlayAnchor`: no anchor above this one shows an overlay.
    fn is_root_overlay_anchor(&self) -> bool {
        self.parent().is_none_or(|parent| parent.is_group)
    }

    /// `_RawMenuAnchorState.open`: an open menu closes before it reopens,
    /// opening closes the open siblings, and a root overlay anchor focuses
    /// its `childFocusNode`.
    fn open(&self, position: Option<Offset>) {
        if self.is_group {
            return;
        }
        if self.open.get() {
            self.close();
        }
        if let Some(parent) = self.parent() {
            parent.close_children();
        }
        self.position.set(position);
        self.open.set(true);
        self.child_changed_open_state();
        self.rebuild();
        if self.is_root_overlay_anchor() {
            let focus = self.child_focus_node.borrow().clone();
            if let Some(focus) = focus {
                focus.request_focus();
            }
        }
        let on_open = self.callbacks.borrow().on_open.clone();
        if let Some(on_open) = on_open {
            on_open();
        }
    }

    /// `close`: closes the submenus first.
    fn close(&self) {
        if self.is_group {
            self.close_children();
            return;
        }
        if !self.open.get() {
            return;
        }
        self.close_children();
        self.open.set(false);
        self.child_changed_open_state();
        self.rebuild();
        let on_close = self.callbacks.borrow().on_close.clone();
        if let Some(on_close) = on_close {
            on_close();
        }
    }

    /// `requestOpen`: asks `onOpenRequested`, which shows the overlay
    /// when it is ready; without it the menu opens at once.
    fn request_open(self: &Rc<Self>, position: Option<Offset>) {
        let requested = self.callbacks.borrow().on_open_requested.clone();
        match requested {
            Some(requested) => {
                let weak = Rc::downgrade(self);
                requested(
                    position,
                    Rc::new(move || {
                        if let Some(node) = weak.upgrade() {
                            node.open(position);
                        }
                    }),
                );
            }
            None => self.open(position),
        }
    }

    /// `handleOutsideTap`: a tap outside the menu tree closes this menu's
    /// submenus. The menu itself closes only if its overlay asks, as
    /// Flutter's does from a [`TapRegion`] in `tapRegionGroupId`.
    fn handle_outside_tap(&self) {
        if self.is_open() {
            self.close_children();
        }
    }

    /// `didChangeDependencies` of a root: the menus close when the view
    /// changes size.
    fn track_view_size(self: &Rc<Self>, size: Size) {
        if self.parent().is_some() {
            return;
        }
        let previous = self.view_size.replace(Some(size));
        if previous.is_some_and(|previous| previous != size) && self.is_open() {
            self.request_close();
        }
    }

    /// `requestClose`, through `onCloseRequested` as for opening.
    fn request_close(self: &Rc<Self>) {
        if self.is_group {
            self.close();
            return;
        }
        let requested = self.callbacks.borrow().on_close_requested.clone();
        match requested {
            Some(requested) => {
                let weak = Rc::downgrade(self);
                requested(Rc::new(move || {
                    if let Some(node) = weak.upgrade() {
                        node.close();
                    }
                }));
            }
            None => self.close(),
        }
    }
}

/// Flutter's `MenuController`: opens and closes the [`RawMenuAnchor`] or
/// [`RawMenuAnchorGroup`] it is given to. Clones control the same menu.
#[derive(Clone, Default)]
pub struct MenuController {
    anchor: Rc<RefCell<Weak<AnchorNode>>>,
}

impl MenuController {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    fn node(&self) -> Option<Rc<AnchorNode>> {
        self.anchor.borrow().upgrade()
    }

    /// Whether the menu is open; for a group, whether any of its menus is.
    #[must_use]
    pub fn is_open(&self) -> bool {
        self.node().is_some_and(|node| node.is_open())
    }

    /// Opens the menu, at `position` relative to the anchor when given.
    /// Open sibling menus close. A group cannot be opened.
    pub fn open(&self, position: Option<Offset>) {
        if let Some(node) = self.node() {
            node.request_open(position);
        }
    }

    /// Closes the menu and its submenus; for a group, all of its menus.
    pub fn close(&self) {
        if let Some(node) = self.node() {
            node.request_close();
        }
    }

    /// Closes the submenus and leaves this menu open.
    pub fn close_children(&self) {
        if let Some(node) = self.node() {
            node.close_children();
        }
    }

    /// `MenuController.maybeOf`: the controller of the nearest enclosing
    /// anchor or group.
    #[must_use]
    pub fn maybe_of(context: &BuildContext<'_>) -> Option<Self> {
        context.depend_on::<AnchorScope>().map(|scope| Self {
            anchor: Rc::new(RefCell::new(Rc::downgrade(&scope.0))),
        })
    }
}

/// The nearest anchor, for nested anchors to attach to.
#[derive(Clone)]
struct AnchorScope(Rc<AnchorNode>);

struct AnchorState(Rc<AnchorNode>);

impl RetainedState for AnchorState {
    fn rebind_revision(&self, revision: Rc<Cell<u64>>) {
        *self.0.revision.borrow_mut() = revision;
    }
}

/// Builds the anchor from the context, its controller and the optional child.
pub type RawMenuAnchorChildBuilder =
    Rc<dyn for<'a> Fn(&BuildContext<'a>, &MenuController, Option<Widget>) -> Widget>;
/// Builds the open menu's overlay.
pub type RawMenuAnchorOverlayBuilder =
    Rc<dyn for<'a> Fn(&BuildContext<'a>, &RawMenuOverlayInfo) -> Widget>;

/// Flutter's `RawMenuAnchor`: an anchor whose menu overlay the application
/// builds and positions from a [`RawMenuOverlayInfo`]. Anchors built inside
/// an open menu are its submenus: they close with it, and opening one
/// closes its open siblings. Escape closes the whole tree. A tap outside the
/// tree closes the submenus; the overlay closes the menu itself by wrapping
/// its panel in a [`TapRegion`] of [`RawMenuOverlayInfo::tap_region_group_id`]
/// whose `on_tap_outside` calls [`MenuController::close`].
pub struct RawMenuAnchor {
    controller: MenuController,
    overlay_builder: RawMenuAnchorOverlayBuilder,
    builder: Option<RawMenuAnchorChildBuilder>,
    child: Option<Widget>,
    consume_outside_taps: bool,
    use_root_overlay: bool,
    child_focus_node: Option<FocusNode>,
    callbacks: Rc<CallbackSet>,
}

#[derive(Default)]
struct CallbackSet {
    on_open: Option<Callback>,
    on_close: Option<Callback>,
    on_open_requested: Option<MenuOpenRequested>,
    on_close_requested: Option<MenuCloseRequested>,
}

impl RawMenuAnchor {
    #[must_use]
    pub fn new(
        controller: MenuController,
        overlay_builder: impl for<'a> Fn(&BuildContext<'a>, &RawMenuOverlayInfo) -> Widget + 'static,
    ) -> Self {
        Self {
            controller,
            overlay_builder: Rc::new(overlay_builder),
            builder: None,
            child: None,
            consume_outside_taps: false,
            use_root_overlay: false,
            child_focus_node: None,
            callbacks: Rc::new(CallbackSet::default()),
        }
    }

    /// Builds the anchor; without it the anchor is [`Self::child`].
    #[must_use]
    pub fn builder(
        mut self,
        builder: impl for<'a> Fn(&BuildContext<'a>, &MenuController, Option<Widget>) -> Widget + 'static,
    ) -> Self {
        self.builder = Some(Rc::new(builder));
        self
    }

    #[must_use]
    pub fn child(mut self, child: impl Into<Widget>) -> Self {
        self.child = Some(child.into());
        self
    }

    /// Whether a tap outside the open menu tree reaches nothing else.
    /// Defaults to false.
    #[must_use]
    pub fn consume_outside_taps(mut self, consume: bool) -> Self {
        self.consume_outside_taps = consume;
        self
    }

    /// Shows the overlay above every enclosing transient. Defaults to false.
    #[must_use]
    pub fn use_root_overlay(mut self, root: bool) -> Self {
        self.use_root_overlay = root;
        self
    }

    /// `childFocusNode`: the node of the widget that takes focus when the
    /// menu opens. A root menu, or one directly in a group, requests it on
    /// opening; without it the anchor does not retain focus.
    #[must_use]
    pub fn child_focus_node(mut self, node: FocusNode) -> Self {
        self.child_focus_node = Some(node);
        self
    }

    fn callbacks_mut(&mut self) -> &mut CallbackSet {
        Rc::get_mut(&mut self.callbacks).expect("callbacks are set before conversion")
    }

    /// Called after the menu opens.
    #[must_use]
    pub fn on_open(mut self, callback: impl Fn() + 'static) -> Self {
        self.callbacks_mut().on_open = Some(Rc::new(callback));
        self
    }

    /// Called after the menu closes.
    #[must_use]
    pub fn on_close(mut self, callback: impl Fn() + 'static) -> Self {
        self.callbacks_mut().on_close = Some(Rc::new(callback));
        self
    }

    /// Called instead of opening; calling the given callback shows the
    /// overlay, so an opening animation can run first or the open can be
    /// refused.
    #[must_use]
    pub fn on_open_requested(
        mut self,
        callback: impl Fn(Option<Offset>, Rc<dyn Fn()>) + 'static,
    ) -> Self {
        self.callbacks_mut().on_open_requested = Some(Rc::new(callback));
        self
    }

    /// Called instead of closing; calling the given callback hides the
    /// overlay, so a closing animation can run first.
    #[must_use]
    pub fn on_close_requested(mut self, callback: impl Fn(Rc<dyn Fn()>) + 'static) -> Self {
        self.callbacks_mut().on_close_requested = Some(Rc::new(callback));
        self
    }

    fn build(&self, node: &Rc<AnchorNode>, context: &BuildContext<'_>) -> Widget {
        node.attach(context.depend_on::<AnchorScope>().map(|scope| scope.0));
        *self.controller.anchor.borrow_mut() = Rc::downgrade(node);
        node.child_focus_node.replace(self.child_focus_node.clone());
        *node.callbacks.borrow_mut() = Callbacks {
            on_open: self.callbacks.on_open.clone(),
            on_close: self.callbacks.on_close.clone(),
            on_open_requested: self.callbacks.on_open_requested.clone(),
            on_close_requested: self.callbacks.on_close_requested.clone(),
        };
        node.track_view_size(context.viewport());
        let root = node.root();
        let group = TapRegionGroupId::Numeric(root.group_id);
        let anchor = match &self.builder {
            Some(builder) => builder(context, &self.controller, self.child.clone()),
            None => self
                .child
                .clone()
                .unwrap_or_else(|| crate::SizedBox::shrink().into()),
        };
        // Every anchor and overlay of the tree is in the root's group, so a
        // tap is outside only when it misses them all.
        let weak = Rc::downgrade(node);
        let anchor = TapRegion::new(anchor)
            .group_id(group.clone())
            .consume_outside_taps(root.is_open() && self.consume_outside_taps)
            .on_tap_outside(move |_| {
                if let Some(node) = weak.upgrade() {
                    node.handle_outside_tap();
                }
            });
        let overlay_builder = self.overlay_builder.clone();
        let position = node.position.get();
        let overlay_group = group.clone();
        let escape_root = Rc::downgrade(&root);
        let portal = OverlayPortal::new(anchor)
            .show(node.open.get())
            .role(TransientRole::Menu)
            .root_overlay(self.use_root_overlay)
            .dismiss_policy(TransientDismissPolicy {
                outside_pointer: false,
                escape: true,
                focus_loss: false,
                parent_deactivation: true,
            })
            .on_dismiss(move |reason| {
                if reason == TransientDismissReason::Escape
                    && let Some(root) = escape_root.upgrade()
                {
                    root.request_close();
                }
            })
            .overlay_child_layout_builder(move |context, layout: OverlayChildLayoutInfo| {
                let info = RawMenuOverlayInfo {
                    anchor_rect: layout.anchor_rect,
                    overlay_size: layout.overlay_size,
                    position,
                    tap_region_group_id: overlay_group.clone(),
                };
                overlay_builder(context, &info)
            });
        Widget::environment_scope(AnchorScope(node.clone()), portal.into())
    }
}

impl From<RawMenuAnchor> for Widget {
    fn from(value: RawMenuAnchor) -> Self {
        Widget::retained_builder(
            |revision| AnchorState(AnchorNode::new(false, revision)),
            move |state, context, _| value.build(&state.0, context),
            false,
        )
    }
}

/// Flutter's `RawMenuAnchorGroup`: groups the anchors below it, as a menu
/// bar does, so that opening one menu closes the others. Its controller
/// reports whether any menu is open and closes them all; it cannot open.
pub struct RawMenuAnchorGroup {
    controller: MenuController,
    child: Widget,
}

impl RawMenuAnchorGroup {
    #[must_use]
    pub fn new(controller: MenuController, child: impl Into<Widget>) -> Self {
        Self {
            controller,
            child: child.into(),
        }
    }
}

impl From<RawMenuAnchorGroup> for Widget {
    fn from(value: RawMenuAnchorGroup) -> Self {
        Widget::retained_builder(
            |revision| AnchorState(AnchorNode::new(true, revision)),
            move |state, context, _| {
                let node = &state.0;
                node.attach(context.depend_on::<AnchorScope>().map(|scope| scope.0));
                *value.controller.anchor.borrow_mut() = Rc::downgrade(node);
                node.track_view_size(context.viewport());
                let root = node.root();
                // A tap outside the tree closes the group's menus.
                let weak = Rc::downgrade(node);
                let region = TapRegion::new(value.child.clone())
                    .group_id(TapRegionGroupId::Numeric(root.group_id))
                    .on_tap_outside(move |_| {
                        if let Some(node) = weak.upgrade() {
                            node.handle_outside_tap();
                        }
                    });
                Widget::environment_scope(AnchorScope(node.clone()), region.into())
            },
            false,
        )
    }
}
