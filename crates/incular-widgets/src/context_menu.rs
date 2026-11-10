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
    rc::Rc,
};

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
            hide,
        }
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
    menu: RefCell<Option<TextContextMenu>>,
    revision: RefCell<Rc<Cell<u64>>>,
}

impl RetainedState for TextContextMenuState {
    fn rebind_revision(&self, revision: Rc<Cell<u64>>) {
        *self.revision.borrow_mut() = revision;
    }
}

impl TextContextMenuState {
    fn new(revision: Rc<Cell<u64>>) -> Self {
        Self {
            menu: RefCell::new(None),
            revision: RefCell::new(revision),
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
        let weak = Rc::downgrade(self);
        let hide: Rc<dyn Fn()> = Rc::new(move || {
            if let Some(state) = weak.upgrade() {
                state.hide();
            }
        });
        let button_items = button_items
            .into_iter()
            .map(|item| {
                let Some(on_pressed) = item.on_pressed.clone() else {
                    return item;
                };
                let hide = hide.clone();
                ContextMenuButtonItem {
                    on_pressed: Some(Rc::new(move || {
                        hide();
                        on_pressed();
                    })),
                    ..item
                }
            })
            .collect();
        *self.menu.borrow_mut() = Some(TextContextMenu::new(anchors, button_items, hide));
        self.bump();
        true
    }

    /// Closes the menu. Returns whether one was showing.
    pub fn hide(&self) -> bool {
        let was_showing = self.menu.borrow_mut().take().is_some();
        if was_showing {
            self.bump();
        }
        was_showing
    }

    /// The open menu.
    #[must_use]
    pub fn menu(&self) -> Option<TextContextMenu> {
        self.menu.borrow().clone()
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
    Widget::retained_builder(
        TextContextMenuState::new,
        move |state, _, _| {
            let menu = state.menu();
            let mut portal = OverlayPortal::new(SizedBox::shrink())
                .role(TransientRole::ContextMenu)
                .show(menu.is_some());
            // The dismiss hook and overlay are only needed while the menu
            // shows; a hidden menu costs a field no closures per build.
            if let Some(menu) = menu {
                let weak = Rc::downgrade(state);
                let builder = builder.clone();
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
                    .overlay_child_layout_builder(move |context, _| builder(context, &menu));
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
