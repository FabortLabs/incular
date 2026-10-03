//! Material menu and dropdown components.
//!
//! This module owns the Material vocabulary and default composition for menus.
//! Hit testing, retained overlays, buttons, text editing, and type-ahead state
//! remain in the lower-level crates.  The types intentionally accept ordinary
//! [`incular_widgets::Widget`] values so an application can use custom menu rows and icons
//! without opting into a second menu renderer.

mod anchor;
mod controls;
mod dropdown;
mod popup;
mod style;
mod vocabulary;

pub use anchor::MenuAnchor;
pub use controls::{MenuBar, MenuController, MenuItemButton, SubmenuButton};
pub use dropdown::{
    DropdownButton, DropdownButtonBuilder, DropdownButtonFormField, DropdownButtonHideUnderline,
    DropdownMenu, DropdownMenuDecorationBuilder, DropdownMenuEntry, DropdownMenuFormField,
    DropdownMenuItem, FilterCallback, SearchCallback,
};
pub use popup::{
    CheckedPopupMenuItem, PopupMenuButton, PopupMenuDivider, PopupMenuEntry, PopupMenuItem,
};
pub use style::{DropdownMenuThemeData, MenuStyle, MenuThemeData, PopupMenuThemeData};
pub use vocabulary::*;
