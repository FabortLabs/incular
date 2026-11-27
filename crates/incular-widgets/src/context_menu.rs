//! Flutter's widgets-library context menu vocabulary for selectable and
//! editable text: `ContextMenuButtonType`, `ContextMenuButtonItem`,
//! `TextSelectionToolbarAnchors` and `EditableText.contextMenuBuilder`.
//!
//! The widgets layer only decides *when* a menu is shown and *which* buttons
//! it offers. How it looks is the builder's job; Material and Controls
//! install their own desktop toolbars, while a bare [`EditableText`]
//! (like Flutter's) has no builder and therefore no menu.
//!
//! [`EditableText`]: crate::EditableText

use crate::{
    BuildContext, OverlayPortal, Positioned, SingleChildLayoutDelegate, SizedBox, Stack, StackFit,
    TransientRole, Widget, tree::RetainedState,
};
use incular_config::Constraints;
use incular_core::{Offset, Rect, Size};
use std::{
    cell::{Cell, RefCell},
    fmt,
    rc::{Rc, Weak},
};

struct ActiveContextMenu {
    owner: Weak<()>,
    close: Rc<dyn Fn()>,
}

thread_local! {
    static ACTIVE_CONTEXT_MENU: RefCell<Option<ActiveContextMenu>> = const { RefCell::new(None) };
}

fn claim_context_menu(owner: &Rc<()>, close: Rc<dyn Fn()>) {
    let owner = Rc::downgrade(owner);
    let previous = ACTIVE_CONTEXT_MENU.with(|slot| {
        let mut slot = slot.borrow_mut();
        if slot.as_ref().is_some_and(|active| active.owner.ptr_eq(&owner)) {
            return None;
        }
        slot.replace(ActiveContextMenu { owner, close })
    });
    if let Some(previous) = previous {
        (previous.close)();
    }
}

fn release_context_menu(owner: &Rc<()>) {
    let owner = Rc::downgrade(owner);
    let removed = ACTIVE_CONTEXT_MENU.try_with(|slot| {
        let mut slot = slot.borrow_mut();
        if slot.as_ref().is_some_and(|active| active.owner.ptr_eq(&owner)) {
            slot.take()
        } else {
            None
        }
    });
    drop(removed);
}

type MenuBuilder = Rc<dyn for<'a> Fn(&BuildContext<'a>) -> Widget>;

#[derive(Default)]
struct ContextMenuControllerState {
    owner: Rc<()>,
    revision: Rc<Cell<u64>>,
    builder: RefCell<Option<MenuBuilder>>,
    entry: Cell<u64>,
    on_remove: RefCell<Option<Rc<dyn Fn()>>>,
}

impl ContextMenuControllerState {
    fn invalidate(&self) {
        self.revision.set(self.revision.get().wrapping_add(1));
        incular_animation::scheduler::schedule_frame();
    }

    fn remove(&self) -> bool {
        let Some(builder) = self.builder.borrow_mut().take() else {
            return false;
        };
        release_context_menu(&self.owner);
        self.invalidate();
        let callback = self.on_remove.borrow().clone();
        drop(builder);
        if let Some(callback) = callback {
            callback();
        }
        true
    }
}

impl Drop for ContextMenuControllerState {
    fn drop(&mut self) {
        release_context_menu(&self.owner);
    }
}

/// Hosts any widget as the current context menu. A controller's [host](Self::host)
/// supplies the retained overlay and inherited context; only one controller
/// or editable text toolbar is visible on the UI thread at a time.
#[derive(Clone, Default)]
pub struct ContextMenuController(Rc<ContextMenuControllerState>);

impl ContextMenuController {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the notification fired when this menu is removed or replaced.
    #[must_use]
    pub fn on_remove(self, callback: impl Fn() + 'static) -> Self {
        self.0.on_remove.replace(Some(Rc::new(callback)));
        self
    }

    /// Keeps the anchor unchanged and attaches the menu's root overlay.
    /// Mount one host per controller.
    #[must_use]
    pub fn host(&self, child: impl Into<Widget>) -> Widget {
        let child = child.into();
        let state = self.0.clone();
        Widget::stateful_builder(state.revision.clone(), move |_| {
            let builder = state.builder.borrow().clone();
            let mut portal = OverlayPortal::new(child.clone())
                .role(TransientRole::ContextMenu)
                .root_overlay(true)
                .show(builder.is_some());
            if let Some(builder) = builder {
                let weak = Rc::downgrade(&state);
                let content: Widget = crate::Builder::new(move |context| builder(context)).into();
                portal = portal
                    .overlay_child(content.with_key(state.entry.get()))
                    .on_dismiss(move |_| {
                        if let Some(state) = weak.upgrade() {
                            state.remove();
                        }
                    });
            }
            portal.into()
        })
    }

    /// Shows a menu on the next build. Replacing the builder while open
    /// preserves the existing overlay entry and its retained child state.
    pub fn show(&self, builder: impl for<'a> Fn(&BuildContext<'a>) -> Widget + 'static) {
        let previous = self.0.builder.replace(Some(Rc::new(builder)));
        if previous.is_none() {
            self.0.entry.set(self.0.entry.get().wrapping_add(1));
            let weak = Rc::downgrade(&self.0);
            claim_context_menu(
                &self.0.owner,
                Rc::new(move || {
                    if let Some(state) = weak.upgrade() {
                        state.remove();
                    }
                }),
            );
        }
        self.0.invalidate();
    }

    #[must_use]
    pub fn is_shown(&self) -> bool {
        self.0.builder.borrow().is_some()
    }

    /// Schedules a rebuild of the visible menu, returning false if hidden.
    pub fn mark_needs_build(&self) -> bool {
        if !self.is_shown() {
            return false;
        }
        self.0.invalidate();
        true
    }

    /// Removes only this controller's menu.
    pub fn remove(&self) -> bool {
        self.0.remove()
    }

    /// Removes the current context menu, including a built-in toolbar.
    pub fn remove_any() -> bool {
        let menu = ACTIVE_CONTEXT_MENU.with(|slot| slot.borrow_mut().take());
        if let Some(menu) = menu {
            (menu.close)();
            true
        } else {
            false
        }
    }
}

/// The type of a context menu button, which decides its default label
/// (Flutter's `ContextMenuButtonType`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ContextMenuButtonType {
    Cut,
    Copy,
    Paste,
    SelectAll,
    Delete,
    LookUp,
    SearchWeb,
    Share,
    LiveTextInput,
    /// A button with an application-supplied label.
    Custom,
}

impl ContextMenuButtonType {
    /// The English label Flutter's `DefaultMaterialLocalizations` and
    /// `DefaultCupertinoLocalizations` give this button. `Custom` has none.
    #[must_use]
    pub const fn default_label(self) -> Option<&'static str> {
        Some(match self {
            Self::Cut => "Cut",
            Self::Copy => "Copy",
            Self::Paste => "Paste",
            Self::SelectAll => "Select all",
            Self::Delete => "Delete",
            Self::LookUp => "Look Up",
            Self::SearchWeb => "Search Web",
            Self::Share => "Share",
            Self::LiveTextInput => "Scan text",
            Self::Custom => return None,
        })
    }
}

/// One button of a context menu (Flutter's `ContextMenuButtonItem`).
#[derive(Clone)]
pub struct ContextMenuButtonItem {
    pub button_type: ContextMenuButtonType,
    /// Overrides the type's default label. Required for `Custom`.
    pub label: Option<String>,
    /// `None` shows a disabled button.
    pub on_pressed: Option<Rc<dyn Fn()>>,
}

impl ContextMenuButtonItem {
    #[must_use]
    pub fn new(button_type: ContextMenuButtonType, on_pressed: impl Fn() + 'static) -> Self {
        Self {
            button_type,
            label: None,
            on_pressed: Some(Rc::new(on_pressed)),
        }
    }

    /// A `Custom` button with its own label.
    #[must_use]
    pub fn custom(label: impl Into<String>, on_pressed: impl Fn() + 'static) -> Self {
        Self {
            button_type: ContextMenuButtonType::Custom,
            label: Some(label.into()),
            on_pressed: Some(Rc::new(on_pressed)),
        }
    }

    #[must_use]
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// The label to show: the explicit one, else the type's default.
    #[must_use]
    pub fn resolved_label(&self) -> String {
        self.label
            .clone()
            .or_else(|| self.button_type.default_label().map(str::to_owned))
            .unwrap_or_default()
    }

    /// Runs the button's callback, if it has one.
    pub fn press(&self) {
        if let Some(on_pressed) = &self.on_pressed {
            on_pressed();
        }
    }
}

impl fmt::Debug for ContextMenuButtonItem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ContextMenuButtonItem")
            .field("button_type", &self.button_type)
            .field("label", &self.label)
            .field("enabled", &self.on_pressed.is_some())
            .finish()
    }
}

impl PartialEq for ContextMenuButtonItem {
    fn eq(&self, other: &Self) -> bool {
        self.button_type == other.button_type
            && self.label == other.label
            && crate::util::same_rc(&self.on_pressed, &other.on_pressed)
    }
}

/// Where a text selection toolbar points, in the overlay's coordinates
/// (Flutter's `TextSelectionToolbarAnchors`). A secondary click anchors at
/// the click with no secondary anchor.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextSelectionToolbarAnchors {
    pub primary_anchor: Offset,
    pub secondary_anchor: Option<Offset>,
}

impl TextSelectionToolbarAnchors {
    #[must_use]
    pub const fn at(primary_anchor: Offset) -> Self {
        Self {
            primary_anchor,
            secondary_anchor: None,
        }
    }
}

/// What a context menu builder receives: Flutter passes the
/// `EditableTextState` and builders read `contextMenuAnchors` and
/// `contextMenuButtonItems` from it.
#[derive(Clone)]
pub struct TextContextMenu {
    pub anchors: TextSelectionToolbarAnchors,
    pub button_items: Vec<ContextMenuButtonItem>,
    spell_check_span: Option<incular_platform::SuggestionSpan>,
    hide: Rc<dyn Fn()>,
}

impl TextContextMenu {
    #[doc(hidden)]
    #[must_use]
    pub fn new(
        anchors: TextSelectionToolbarAnchors,
        button_items: Vec<ContextMenuButtonItem>,
        hide: Rc<dyn Fn()>,
    ) -> Self {
        Self {
            anchors,
            button_items,
            spell_check_span: None,
            hide,
        }
    }

    /// The misspelled range and replacements for a suggestions toolbar.
    #[must_use]
    pub fn spell_check_span(&self) -> Option<&incular_platform::SuggestionSpan> {
        self.spell_check_span.as_ref()
    }

    /// Closes the menu (`EditableTextState.hideToolbar`).
    pub fn hide(&self) {
        (self.hide)();
    }
}

impl fmt::Debug for TextContextMenu {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TextContextMenu")
            .field("anchors", &self.anchors)
            .field("button_items", &self.button_items)
            .finish_non_exhaustive()
    }
}

/// Flutter's `EditableTextContextMenuBuilder`. The returned widget fills
/// the overlay and positions itself from [`TextContextMenu::anchors`], as
/// Flutter's toolbars do with `CustomSingleChildLayout`.
pub type EditableTextContextMenuBuilder =
    Rc<dyn for<'a> Fn(&BuildContext<'a>, &TextContextMenu) -> Widget>;

/// The visible menu of one editable or selectable text element. The runtime
/// opens it on a secondary click; the menu's own buttons and outside clicks
/// close it.
#[doc(hidden)]
pub struct TextContextMenuState {
    owner: Rc<()>,
    menu: RefCell<Option<Rc<TextContextMenu>>>,
    revision: RefCell<Rc<Cell<u64>>>,
    incarnation: Cell<u64>,
}

impl RetainedState for TextContextMenuState {
    fn rebind_revision(&self, revision: Rc<Cell<u64>>) {
        *self.revision.borrow_mut() = revision;
    }
}

impl Drop for TextContextMenuState {
    fn drop(&mut self) {
        release_context_menu(&self.owner);
    }
}

impl TextContextMenuState {
    fn new(revision: Rc<Cell<u64>>) -> Self {
        Self {
            owner: Rc::new(()),
            menu: RefCell::new(None),
            revision: RefCell::new(revision),
            incarnation: Cell::new(0),
        }
    }

    fn bump(&self) {
        let revision = self.revision.borrow().clone();
        revision.set(revision.get().wrapping_add(1));
    }

    /// Whether the menu is showing.
    #[must_use]
    pub fn is_showing(&self) -> bool {
        self.menu.borrow().is_some()
    }

    /// Shows `button_items` at `anchors`, replacing any open menu. Each
    /// button closes the menu before running its callback, as Flutter's
    /// desktop toolbars do. Returns false when there is nothing to show.
    pub fn show(
        self: &Rc<Self>,
        anchors: TextSelectionToolbarAnchors,
        button_items: Vec<ContextMenuButtonItem>,
    ) -> bool {
        if button_items.is_empty() {
            return self.hide();
        }
        self.show_menu(anchors, button_items, None)
    }

    /// Shows a spell-check toolbar using the same retained selection portal.
    pub fn show_spell_check(
        self: &Rc<Self>,
        anchors: TextSelectionToolbarAnchors,
        button_items: Vec<ContextMenuButtonItem>,
        span: incular_platform::SuggestionSpan,
    ) -> bool {
        self.show_menu(anchors, button_items, Some(span))
    }

    /// Whether the open toolbar is for spelling rather than editing commands.
    #[must_use]
    pub fn is_spell_check_showing(&self) -> bool {
        self.menu.borrow().as_ref().is_some_and(|menu| menu.spell_check_span.is_some())
    }

    fn show_menu(
        self: &Rc<Self>,
        anchors: TextSelectionToolbarAnchors,
        button_items: Vec<ContextMenuButtonItem>,
        spell_check_span: Option<incular_platform::SuggestionSpan>,
    ) -> bool {
        let incarnation = self.incarnation.get().wrapping_add(1);
        self.incarnation.set(incarnation);
        let spelling = spell_check_span.is_some();
        let weak = Rc::downgrade(self);
        let hide: Rc<dyn Fn()> = Rc::new({
            let weak = weak.clone();
            move || {
                if let Some(state) = weak.upgrade()
                    && (!spelling || state.incarnation.get() == incarnation)
                {
                    state.hide();
                }
            }
        });
        let button_items = button_items
            .into_iter()
            .map(|item| {
                let Some(on_pressed) = item.on_pressed.clone() else {
                    return item;
                };
                let hide = hide.clone();
                let weak = weak.clone();
                ContextMenuButtonItem {
                    on_pressed: Some(Rc::new(move || {
                        // A retained suggestion can outlive the menu, but a
                        // dismissed overlay must not edit the field.
                        if spelling && !weak.upgrade().is_some_and(|state| {
                            state.incarnation.get() == incarnation && state.is_spell_check_showing()
                        }) {
                            return;
                        }
                        hide();
                        on_pressed();
                    })),
                    ..item
                }
            })
            .collect();
        *self.menu.borrow_mut() = Some(Rc::new(TextContextMenu {
            anchors,
            button_items,
            spell_check_span,
            hide,
        }));
        claim_context_menu(
            &self.owner,
            Rc::new(move || {
                if let Some(state) = weak.upgrade() {
                    state.hide();
                }
            }),
        );
        self.bump();
        true
    }

    /// Closes the menu. Returns whether one was showing.
    pub fn hide(&self) -> bool {
        let menu = self.menu.borrow_mut().take();
        if menu.is_some() {
            release_context_menu(&self.owner);
            self.bump();
            incular_animation::scheduler::schedule_frame();
        }
        menu.is_some()
    }

    /// The open menu.
    #[must_use]
    pub fn menu(&self) -> Option<TextContextMenu> {
        self.menu.borrow().as_deref().cloned()
    }
}

/// Lays `child` out unchanged and keeps a zero-size portal beside it that
/// presents the builder's menu while `state` shows one. The child's position
/// in the tree never depends on whether the menu is open, so an editor keeps
/// its element (focus, scroll offset, composition) when the menu toggles.
pub(crate) fn with_text_context_menu(
    child: impl Fn(Rc<TextContextMenuState>) -> Widget + 'static,
    builder: EditableTextContextMenuBuilder,
) -> Widget {
    with_editable_toolbars(child, Some(builder), None)
}

pub(crate) fn with_editable_toolbars(
    child: impl Fn(Rc<TextContextMenuState>) -> Widget + 'static,
    context_builder: Option<EditableTextContextMenuBuilder>,
    spell_builder: Option<EditableTextContextMenuBuilder>,
) -> Widget {
    Widget::retained_builder(
        TextContextMenuState::new,
        move |state, _, _| {
            let menu_and_builder = state.menu.borrow().clone().and_then(|menu| {
                let builder = if menu.spell_check_span.is_some() {
                    spell_builder.as_ref()
                } else {
                    context_builder.as_ref()
                };
                builder.map(|builder| (menu, builder.clone()))
            });
            let mut portal = OverlayPortal::new(SizedBox::shrink())
                .role(TransientRole::ContextMenu)
                .show(menu_and_builder.is_some());
            // The dismiss hook and overlay are only needed while the menu
            // shows; a hidden menu costs a field no closures per build.
            if let Some((menu, builder)) = menu_and_builder {
                let weak = Rc::downgrade(state);
                portal = portal
                    .on_dismiss(move |_| {
                        if let Some(state) = weak.upgrade() {
                            state.hide();
                        }
                    })
                    .anchor_rect(Rect::from_origin_size(
                        menu.anchors.primary_anchor,
                        Size::ZERO,
                    ))
                    // Flutter `_SelectionToolbarWrapper`: a press on the menu
                    // is not a tap outside the field, which keeps its focus.
                    .overlay_child_layout_builder(move |context, _| {
                        crate::TextFieldTapRegion::new(builder(context, &menu)).into()
                    });
            }
            Stack::new([
                child(state.clone()),
                Positioned::builder()
                    .left(0.0)
                    .top(0.0)
                    .width(0.0)
                    .height(0.0)
                    .child(portal)
                    .build()
                    .into(),
            ])
            .fit(StackFit::Passthrough)
            .clip_behavior(incular_config::Clip::None)
            .into()
        },
        false,
    )
}

/// Flutter's `DesktopTextSelectionToolbarLayoutDelegate`: the toolbar's
/// top-left corner sits at the anchor, shifted back inside the overlay when
/// it would overhang the right or bottom edge.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DesktopTextSelectionToolbarLayoutDelegate {
    pub anchor: Offset,
}

impl SingleChildLayoutDelegate for DesktopTextSelectionToolbarLayoutDelegate {
    fn get_constraints_for_child(&self, constraints: Constraints) -> Constraints {
        constraints.loosen()
    }

    fn get_position_for_child(&self, size: Size, child_size: Size) -> Offset {
        let overhang_x = self.anchor.x + child_size.width - size.width;
        let overhang_y = self.anchor.y + child_size.height - size.height;
        Offset::new(
            if overhang_x > 0.0 {
                self.anchor.x - overhang_x
            } else {
                self.anchor.x
            },
            if overhang_y > 0.0 {
                self.anchor.y - overhang_y
            } else {
                self.anchor.y
            },
        )
    }

    fn should_relayout(&self, old: &Self) -> bool {
        self.anchor != old.anchor
    }
}

/// Flutter's `TextSelectionToolbarLayoutDelegate`, the touch toolbars'
/// placement: centered on `anchor_above` when the toolbar fits above it,
/// otherwise on `anchor_below`, and moved sideways to stay inside the
/// overlay.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextSelectionToolbarLayoutDelegate {
    /// Where the toolbar's bottom center goes when it fits above.
    pub anchor_above: Offset,
    /// Where the toolbar's top center goes otherwise.
    pub anchor_below: Offset,
    /// Whether the toolbar goes above; `None` decides by whether its height
    /// fits above `anchor_above`.
    pub fits_above: Option<bool>,
}

impl TextSelectionToolbarLayoutDelegate {
    /// Flutter `centerOn`: `width` centered on `position`, kept in
    /// `0..max`.
    #[must_use]
    pub fn center_on(position: f32, width: f32, max: f32) -> f32 {
        if position - width / 2.0 < 0.0 {
            0.0
        } else if position + width / 2.0 > max {
            max - width
        } else {
            position - width / 2.0
        }
    }
}

impl SingleChildLayoutDelegate for TextSelectionToolbarLayoutDelegate {
    fn get_constraints_for_child(&self, constraints: Constraints) -> Constraints {
        constraints.loosen()
    }

    fn get_position_for_child(&self, size: Size, child_size: Size) -> Offset {
        let fits_above = self
            .fits_above
            .unwrap_or(self.anchor_above.y >= child_size.height);
        let anchor = if fits_above {
            self.anchor_above
        } else {
            self.anchor_below
        };
        Offset::new(
            Self::center_on(anchor.x, child_size.width, size.width),
            if fits_above {
                (anchor.y - child_size.height).max(0.0)
            } else {
                anchor.y
            },
        )
    }

    fn should_relayout(&self, old: &Self) -> bool {
        self != old
    }
}
