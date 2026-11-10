//! Breakpoints, responsive values and autoscaling, after the
//! `responsive_framework` package.
//!
//! A [`ResponsiveBreakpoints`] measures the width it is given and publishes
//! the matching [`Breakpoint`]; widgets below read it with
//! [`ResponsiveBreakpoints::of`] and pick values with [`ResponsiveValue`],
//! show or hide with [`ResponsiveVisibility`], switch between a row and a
//! column with [`ResponsiveRowColumn`], or lay out at a fixed design width
//! and scale to fit with [`ResponsiveScaledBox`].

use std::{fmt, rc::Rc};

use incular_config::{Constraints, CrossAxisAlignment, MainAxisAlignment};

use crate::{BuildContext, Column, FittedBox, LayoutBuilder, Row, SizedBox, Widget};

/// The conventional breakpoint names `responsive_framework` uses.
pub const MOBILE: &str = "MOBILE";
pub const TABLET: &str = "TABLET";
pub const DESKTOP: &str = "DESKTOP";
/// `4K` in `responsive_framework`.
pub const UHD: &str = "4K";

/// A named width range, `start` through `end` inclusive.
#[derive(Clone, Debug, PartialEq)]
pub struct Breakpoint {
    pub start: f32,
    pub end: f32,
    pub name: Option<Rc<str>>,
}

impl Breakpoint {
    #[must_use]
    pub fn new(start: f32, end: f32, name: impl Into<Rc<str>>) -> Self {
        Self {
            start,
            end,
            name: Some(name.into()),
        }
    }

    /// `responsive_framework`'s default set: mobile to 450, tablet to 800,
    /// desktop to 1920, 4K above.
    #[must_use]
    pub fn defaults() -> Vec<Self> {
        vec![
            Self::new(0., 450., MOBILE),
            Self::new(451., 800., TABLET),
            Self::new(801., 1920., DESKTOP),
            Self::new(1921., f32::INFINITY, UHD),
        ]
    }

    fn contains(&self, width: f32) -> bool {
        width >= self.start && width <= self.end
    }
}

/// The breakpoint state published by [`ResponsiveBreakpoints`].
#[derive(Clone, Debug, PartialEq)]
pub struct ResponsiveBreakpointsData {
    pub width: f32,
    pub breakpoint: Option<Breakpoint>,
    pub breakpoints: Rc<[Breakpoint]>,
}

impl ResponsiveBreakpointsData {
    /// The data for `width` among `breakpoints`: the last breakpoint that
    /// contains the width wins, as in `responsive_framework`.
    #[must_use]
    pub fn for_width(width: f32, breakpoints: Rc<[Breakpoint]>) -> Self {
        let breakpoint = breakpoints
            .iter()
            .rev()
            .find(|breakpoint| breakpoint.contains(width))
            .cloned();
        Self {
            width,
            breakpoint,
            breakpoints,
        }
    }

    fn named(&self, name: &str) -> Option<&Breakpoint> {
        self.breakpoints
            .iter()
            .find(|breakpoint| breakpoint.name.as_deref() == Some(name))
    }

    #[must_use]
    pub fn name(&self) -> Option<&str> {
        self.breakpoint
            .as_ref()
            .and_then(|breakpoint| breakpoint.name.as_deref())
    }

    /// The active breakpoint is the one named `name`.
    #[must_use]
    pub fn equals(&self, name: &str) -> bool {
        self.name() == Some(name)
    }

    /// The width is past the end of the breakpoint named `name`.
    #[must_use]
    pub fn larger_than(&self, name: &str) -> bool {
        self.named(name)
            .is_some_and(|breakpoint| self.width > breakpoint.end)
    }

    /// The width is before the start of the breakpoint named `name`.
    #[must_use]
    pub fn smaller_than(&self, name: &str) -> bool {
        self.named(name)
            .is_some_and(|breakpoint| self.width < breakpoint.start)
    }

    /// The width is from the start of `from` to the end of `to`.
    #[must_use]
    pub fn between(&self, from: &str, to: &str) -> bool {
        match (self.named(from), self.named(to)) {
            (Some(from), Some(to)) => self.width >= from.start && self.width <= to.end,
            _ => false,
        }
    }

    #[must_use]
    pub fn is_mobile(&self) -> bool {
        self.equals(MOBILE)
    }

    #[must_use]
    pub fn is_tablet(&self) -> bool {
        self.equals(TABLET)
    }

    #[must_use]
    pub fn is_desktop(&self) -> bool {
        self.equals(DESKTOP)
    }

    /// The condition holds at this width.
    #[must_use]
    pub fn matches(&self, condition: &Condition) -> bool {
        match condition {
            Condition::Equals(name) => self.equals(name),
            Condition::LargerThan(name) => self.larger_than(name),
            Condition::SmallerThan(name) => self.smaller_than(name),
            Condition::Between(from, to) => self.between(from, to),
            Condition::WidthAtLeast(width) => self.width >= *width,
            Condition::WidthBelow(width) => self.width < *width,
        }
    }
}

/// Publishes the breakpoint matching its width to its subtree
/// (`ResponsiveBreakpoints.builder`).
#[derive(Clone)]
pub struct ResponsiveBreakpoints {
    breakpoints: Rc<[Breakpoint]>,
    child: Widget,
}

impl ResponsiveBreakpoints {
    #[must_use]
    pub fn new(
        breakpoints: impl IntoIterator<Item = Breakpoint>,
        child: impl Into<Widget>,
    ) -> Self {
        Self {
            breakpoints: breakpoints.into_iter().collect::<Vec<_>>().into(),
            child: child.into(),
        }
    }

    /// With [`Breakpoint::defaults`].
    #[must_use]
    pub fn with_defaults(child: impl Into<Widget>) -> Self {
        Self::new(Breakpoint::defaults(), child)
    }

    /// The nearest breakpoint data (`ResponsiveBreakpoints.of`).
    #[must_use]
    pub fn of(context: &BuildContext<'_>) -> Option<ResponsiveBreakpointsData> {
        context.depend_on::<ResponsiveBreakpointsData>()
    }
}

impl From<ResponsiveBreakpoints> for Widget {
    fn from(value: ResponsiveBreakpoints) -> Self {
        LayoutBuilder::new(move |_, constraints| {
            let width = constraints.max_width();
            let width = if width.is_finite() {
                width
            } else {
                constraints.min_width()
            };
            Widget::environment_scope(
                ResponsiveBreakpointsData::for_width(width, value.breakpoints.clone()),
                value.child.clone(),
            )
        })
        .into()
    }
}

/// A breakpoint condition (`responsive_framework`'s `Condition`).
#[derive(Clone, Debug, PartialEq)]
pub enum Condition {
    Equals(Rc<str>),
    LargerThan(Rc<str>),
    SmallerThan(Rc<str>),
    Between(Rc<str>, Rc<str>),
    /// Holds at or past a raw width, with no named breakpoint.
    WidthAtLeast(f32),
    /// Holds below a raw width.
    WidthBelow(f32),
}

impl Condition {
    #[must_use]
    pub fn equals(name: impl Into<Rc<str>>) -> Self {
        Self::Equals(name.into())
    }

    #[must_use]
    pub fn larger_than(name: impl Into<Rc<str>>) -> Self {
        Self::LargerThan(name.into())
    }

    #[must_use]
    pub fn smaller_than(name: impl Into<Rc<str>>) -> Self {
        Self::SmallerThan(name.into())
    }

    #[must_use]
    pub fn between(from: impl Into<Rc<str>>, to: impl Into<Rc<str>>) -> Self {
        Self::Between(from.into(), to.into())
    }
}

/// A value that depends on the breakpoint (`ResponsiveValue`): the first
/// condition that holds supplies the value, else the default.
#[derive(Clone)]
pub struct ResponsiveValue<T> {
    default_value: T,
    conditional: Vec<(Condition, T)>,
}

impl<T: fmt::Debug> fmt::Debug for ResponsiveValue<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ResponsiveValue")
            .field("default_value", &self.default_value)
            .field("conditional", &self.conditional)
            .finish()
    }
}

impl<T: Clone> ResponsiveValue<T> {
    #[must_use]
    pub fn new(default_value: T) -> Self {
        Self {
            default_value,
            conditional: Vec::new(),
        }
    }

    #[must_use]
    pub fn when(mut self, condition: Condition, value: T) -> Self {
        self.conditional.push((condition, value));
        self
    }

    /// The value for `data`.
    #[must_use]
    pub fn resolve(&self, data: &ResponsiveBreakpointsData) -> T {
        self.conditional
            .iter()
            .find(|(condition, _)| data.matches(condition))
            .map_or_else(|| self.default_value.clone(), |(_, value)| value.clone())
    }

    /// The value for the nearest [`ResponsiveBreakpoints`], or the default
    /// outside one.
    #[must_use]
    pub fn value(&self, context: &BuildContext<'_>) -> T {
        ResponsiveBreakpoints::of(context)
            .map_or_else(|| self.default_value.clone(), |data| self.resolve(&data))
    }
}

/// Shows its child unless a hidden condition holds, and while any visible
/// condition (if given) holds (`ResponsiveVisibility`).
#[derive(Clone)]
pub struct ResponsiveVisibility {
    child: Widget,
    visible: bool,
    visible_conditions: Vec<Condition>,
    hidden_conditions: Vec<Condition>,
    replacement: Option<Widget>,
}

impl ResponsiveVisibility {
    #[must_use]
    pub fn new(child: impl Into<Widget>) -> Self {
        Self {
            child: child.into(),
            visible: true,
            visible_conditions: Vec::new(),
            hidden_conditions: Vec::new(),
            replacement: None,
        }
    }

    /// The visibility when no condition applies.
    #[must_use]
    pub fn visible(mut self, visible: bool) -> Self {
        self.visible = visible;
        self
    }

    #[must_use]
    pub fn visible_when(mut self, condition: Condition) -> Self {
        self.visible_conditions.push(condition);
        self
    }

    #[must_use]
    pub fn hidden_when(mut self, condition: Condition) -> Self {
        self.hidden_conditions.push(condition);
        self
    }

    #[must_use]
    pub fn replacement(mut self, replacement: impl Into<Widget>) -> Self {
        self.replacement = Some(replacement.into());
        self
    }

    fn is_visible(&self, data: Option<&ResponsiveBreakpointsData>) -> bool {
        let Some(data) = data else {
            return self.visible;
        };
        if self
            .hidden_conditions
            .iter()
            .any(|condition| data.matches(condition))
        {
            return false;
        }
        if self
            .visible_conditions
            .iter()
            .any(|condition| data.matches(condition))
        {
            return true;
        }
        self.visible
    }
}

impl From<ResponsiveVisibility> for Widget {
    fn from(value: ResponsiveVisibility) -> Self {
        crate::Builder::new(move |context| {
            if value.is_visible(ResponsiveBreakpoints::of(context).as_ref()) {
                value.child.clone()
            } else {
                value
                    .replacement
                    .clone()
                    .unwrap_or_else(|| SizedBox::shrink().into())
            }
        })
        .into()
    }
}

/// Lays its children out in a row, or in a column while a condition holds
/// (`ResponsiveRowColumn`).
#[derive(Clone)]
pub struct ResponsiveRowColumn {
    children: Vec<Widget>,
    column_when: Vec<Condition>,
    spacing: f32,
    main_axis_alignment: MainAxisAlignment,
    cross_axis_alignment: CrossAxisAlignment,
}

impl ResponsiveRowColumn {
    #[must_use]
    pub fn new(children: impl IntoIterator<Item = impl Into<Widget>>) -> Self {
        Self {
            children: children.into_iter().map(Into::into).collect(),
            column_when: Vec::new(),
            spacing: 0.,
            main_axis_alignment: MainAxisAlignment::Start,
            cross_axis_alignment: CrossAxisAlignment::Center,
        }
    }

    #[must_use]
    pub fn column_when(mut self, condition: Condition) -> Self {
        self.column_when.push(condition);
        self
    }

    #[must_use]
    pub fn spacing(mut self, spacing: f32) -> Self {
        self.spacing = spacing;
        self
    }

    #[must_use]
    pub fn main_axis_alignment(mut self, alignment: MainAxisAlignment) -> Self {
        self.main_axis_alignment = alignment;
        self
    }

    #[must_use]
    pub fn cross_axis_alignment(mut self, alignment: CrossAxisAlignment) -> Self {
        self.cross_axis_alignment = alignment;
        self
    }
}

impl From<ResponsiveRowColumn> for Widget {
    fn from(value: ResponsiveRowColumn) -> Self {
        crate::Builder::new(move |context| {
            let column = ResponsiveBreakpoints::of(context).is_some_and(|data| {
                value
                    .column_when
                    .iter()
                    .any(|condition| data.matches(condition))
            });
            if column {
                Column::new(value.children.clone())
                    .spacing(value.spacing)
                    .main_axis_alignment(value.main_axis_alignment)
                    .cross_axis_alignment(value.cross_axis_alignment)
                    .into()
            } else {
                Row::new(value.children.clone())
                    .spacing(value.spacing)
                    .main_axis_alignment(value.main_axis_alignment)
                    .cross_axis_alignment(value.cross_axis_alignment)
                    .into()
            }
        })
        .into()
    }
}

/// Lays its child out at a fixed design width and scales it to the width
/// it is given (`ResponsiveScaledBox`, the AutoScale behavior).
#[derive(Clone)]
pub struct ResponsiveScaledBox {
    width: f32,
    child: Widget,
}

impl ResponsiveScaledBox {
    #[must_use]
    pub fn new(width: f32, child: impl Into<Widget>) -> Self {
        Self {
            width: width.max(1.),
            child: child.into(),
        }
    }

    /// The scale factor at `available` width.
    #[must_use]
    pub fn scale_for(&self, available: f32) -> f32 {
        available / self.width
    }
}

impl From<ResponsiveScaledBox> for Widget {
    fn from(value: ResponsiveScaledBox) -> Self {
        LayoutBuilder::new(move |_, constraints: Constraints| {
            let available = constraints.max_width();
            if !available.is_finite() {
                return value.child.clone();
            }
            let scale = value.scale_for(available);
            let height = constraints.max_height();
            // The child gets the design width and the height that will scale
            // back to the available height.
            let design_height = if height.is_finite() {
                height / scale
            } else {
                f32::INFINITY
            };
            let mut inner = SizedBox::new()
                .width(value.width)
                .child(value.child.clone());
            if design_height.is_finite() {
                inner = inner.height(design_height);
            }
            FittedBox::new(inner).into()
        })
        .into()
    }
}
