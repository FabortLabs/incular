use std::{collections::HashSet, fmt};

use crate::{Role, SemanticActionKind, SemanticNode, SemanticNodeId, SemanticsTree};

/// The accessibility contract violated by a finished semantic node.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SemanticRoleIssueKind {
    TabWithoutSelectedState,
    EnabledTabWithoutTap,
    ListItemOutsideList,
    EmptyTabList,
    TabListWithNonTabChild,
    EmptyMenu,
    EmptyMenuBar,
    MenuItemOutsideMenu,
    MenuItemCheckboxWithoutCheckedState,
    MenuItemCheckboxOutsideMenu,
    MenuItemRadioWithoutCheckedState,
    MenuItemRadioOutsideMenu,
    LiveAlertOrStatus,
    ProgressBarMissingValueOrBounds,
    SliderMissingValue,
    SliderMissingMinValue,
    SliderMissingMaxValue,
    InvalidRangeNumber,
    InvertedValueRange,
    ValueOutsideRange,
    PercentageOutsideRange,
}

impl fmt::Display for SemanticRoleIssueKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::TabWithoutSelectedState => "A tab needs selected states",
            Self::EnabledTabWithoutTap => "A tab must have a tap action",
            Self::ListItemOutsideList => "A list item must have a list parent",
            Self::EmptyTabList => "a TabBar cannot be empty",
            Self::TabListWithNonTabChild => "Children of TabBar must have the tab role",
            Self::EmptyMenu => "a menu cannot be empty",
            Self::EmptyMenuBar => "a menu bar cannot be empty",
            Self::MenuItemCheckboxWithoutCheckedState => "a menu item checkbox must be checkable",
            Self::MenuItemCheckboxOutsideMenu => {
                "A menu item checkbox must be a child of a menu or a menu bar"
            }
            Self::MenuItemRadioWithoutCheckedState => "a menu item radio must be checkable",
            Self::MenuItemRadioOutsideMenu => {
                "A menu item radio must be a child of a menu or a menu bar"
            }
            Self::MenuItemOutsideMenu => "A menu item must be a child of a menu or a menu bar",
            Self::LiveAlertOrStatus => "An alert or status node cannot also be a live region",
            Self::ProgressBarMissingValueOrBounds => {
                "A progress bar must have a value, a minValue, a maxValue."
            }
            Self::SliderMissingValue => "A slider must have a value",
            Self::SliderMissingMinValue => "A slider must have a minValue",
            Self::SliderMissingMaxValue => "A slider must have a maxValue",
            Self::InvalidRangeNumber => "The value, minValue, and maxValue must be valid numbers",
            Self::InvertedValueRange => "minValue must be less than maxValue",
            Self::ValueOutsideRange => "The value must be between minValue and maxValue",
            Self::PercentageOutsideRange => "The percentage value must be between 0% and 100%",
        })
    }
}

/// A diagnostic on an actual retained semantic node, not a layout widget.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SemanticRoleIssue {
    pub node: SemanticNodeId,
    pub role: Role,
    pub kind: SemanticRoleIssueKind,
}

impl SemanticsTree {
    /// Validates supported role contracts on the emitted semantic forest.
    /// Native widget hosts supply the separate user-action blocking state
    /// through [Self::role_issues_with_action_blocks].
    #[must_use]
    pub fn role_issues(&self) -> Vec<SemanticRoleIssue> {
        self.role_issues_with_action_blocks(|_| false)
    }

    /// Reports the first supported role violation per semantic node.
    /// A blocked tab still needs a selected state, but can omit its tap.
    #[must_use]
    pub fn role_issues_with_action_blocks(
        &self,
        mut blocks_user_actions: impl FnMut(SemanticNodeId) -> bool,
    ) -> Vec<SemanticRoleIssue> {
        use SemanticRoleIssueKind::*;
        let mut reports = Vec::new();
        let mut visited = HashSet::new();
        let mut work: Vec<_> = self
            .roots()
            .iter()
            .rev()
            .map(|&id| (id, None, false))
            .collect();
        while let Some((id, parent, has_menu)) = work.pop() {
            let Some(node) = self.node(id).filter(|_| visited.insert(id)) else {
                continue;
            };
            let issue = match node.role {
                Role::Tab if node.state.declared_selected().is_none() => {
                    Some(TabWithoutSelectedState)
                }
                Role::Tab
                    if node.state.enabled
                        && !node.supports_action(SemanticActionKind::Activate)
                        && !blocks_user_actions(id) =>
                {
                    Some(EnabledTabWithoutTap)
                }
                Role::ListItem if parent != Some(Role::List) => Some(ListItemOutsideList),
                Role::TabList if node.children.is_empty() => Some(EmptyTabList),
                Role::TabList
                    if node.children.iter().any(|&child| {
                        self.node(child).is_none_or(|child| child.role != Role::Tab)
                    }) =>
                {
                    Some(TabListWithNonTabChild)
                }
                Role::Menu if node.children.is_empty() => Some(EmptyMenu),
                Role::MenuBar if node.children.is_empty() => Some(EmptyMenuBar),
                Role::MenuItem if !has_menu => Some(MenuItemOutsideMenu),
                Role::MenuItemCheckbox if node.state.checked.is_none() => {
                    Some(MenuItemCheckboxWithoutCheckedState)
                }
                Role::MenuItemCheckbox if !has_menu => Some(MenuItemCheckboxOutsideMenu),
                Role::MenuItemRadio if node.state.checked.is_none() => {
                    Some(MenuItemRadioWithoutCheckedState)
                }
                Role::MenuItemRadio if !has_menu => Some(MenuItemRadioOutsideMenu),
                Role::Alert | Role::Status if node.state.live_region => Some(LiveAlertOrStatus),
                Role::Slider | Role::ProgressBar => range_issue(node),
                _ => None,
            };
            if let Some(kind) = issue {
                reports.push(SemanticRoleIssue {
                    node: id,
                    role: node.role,
                    kind,
                });
            }
            let has_menu = has_menu || matches!(node.role, Role::Menu | Role::MenuBar);
            work.extend(
                node.children
                    .iter()
                    .rev()
                    .map(|&child| (child, Some(node.role), has_menu)),
            );
        }
        reports
    }
}

#[derive(Clone, Copy)]
enum RangeValue<'a> {
    Text(&'a str),
    Number(f64),
}

impl<'a> RangeValue<'a> {
    fn from_parts(text: Option<&'a str>, number: Option<f64>) -> Option<Self> {
        match text {
            Some("") => None,
            Some(text) => Some(Self::Text(text)),
            None => number.map(Self::Number),
        }
    }

    fn parse(self) -> Option<f64> {
        match self {
            Self::Text(text) => text.trim().parse::<f64>().ok(),
            Self::Number(number) => Some(number),
        }
        .filter(|number| number.is_finite())
    }

    fn parse_value(self, slider: bool) -> Option<(f64, bool)> {
        let Self::Text(text) = self else {
            return self.parse().map(|value| (value, false));
        };
        if slider && text.contains('%') {
            return text
                .replace('%', "")
                .trim()
                .parse::<f64>()
                .ok()
                .filter(|value| value.is_finite())
                .map(|value| (value, true));
        }
        if let Some(percent) = text.strip_suffix('%') {
            return Self::Text(percent).parse().map(|value| (value, true));
        }
        self.parse().map(|value| (value, false))
    }
}

fn range_issue(node: &SemanticNode) -> Option<SemanticRoleIssueKind> {
    use SemanticRoleIssueKind::*;
    let bounds = node.state.value_bounds_text.as_deref();
    let value = RangeValue::from_parts(node.value.as_deref(), node.state.numeric_value);
    let min = RangeValue::from_parts(
        bounds.and_then(|bounds| bounds.min.as_deref()),
        node.state.numeric_min,
    );
    let max = RangeValue::from_parts(
        bounds.and_then(|bounds| bounds.max.as_deref()),
        node.state.numeric_max,
    );
    let slider = node.role == Role::Slider;
    let (Some(value), Some(min), Some(max)) = (value, min, max) else {
        return Some(if !slider {
            ProgressBarMissingValueOrBounds
        } else if value.is_none() {
            SliderMissingValue
        } else if min.is_none() {
            SliderMissingMinValue
        } else {
            SliderMissingMaxValue
        });
    };
    let (Some((value, percent)), Some(min), Some(max)) =
        (value.parse_value(slider), min.parse(), max.parse())
    else {
        return Some(InvalidRangeNumber);
    };
    if min >= max {
        Some(InvertedValueRange)
    } else if percent && !(0. ..=100.).contains(&value) {
        Some(PercentageOutsideRange)
    } else if !percent && !(min..=max).contains(&value) {
        Some(ValueOutsideRange)
    } else {
        None
    }
}
