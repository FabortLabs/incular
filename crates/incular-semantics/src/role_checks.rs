use std::{collections::HashSet, fmt};

use crate::{
    Role, SemanticActionKind, SemanticAnnotationFlag, SemanticNode, SemanticNodeId, SemanticsTree,
};

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
    RadioGroupWithMultipleCheckedChildren,
    LiveAlertOrStatus,
    NestedLandmark,
    RepeatedLandmarkNeedsLabel,
    RegionNeedsLabel,
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
            Self::RadioGroupWithMultipleCheckedChildren => {
                "Radio groups must not have multiple checked children"
            }
            Self::NestedLandmark => "This landmark cannot be contained within another landmark",
            Self::RepeatedLandmarkNeedsLabel => "Repeated landmarks need a label",
            Self::RegionNeedsLabel => {
                "A region role should include a label that describes the purpose of the content."
            }
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
        let mut counts = [0usize; 4];
        let mut unlabeled = Vec::new();
        let mut work: Vec<_> = self
            .roots()
            .iter()
            .rev()
            .map(|&id| (id, None, false, false))
            .collect();
        while let Some((id, parent, has_menu, has_landmark)) = work.pop() {
            let Some(node) = self.node(id).filter(|_| visited.insert(id)) else {
                continue;
            };
            let repeating_landmark = repeatable_landmark_index(node.role);
            if let Some(index) = repeating_landmark {
                counts[index] += 1;
            }
            let has_label = node.label.as_deref().is_some_and(|label| !label.is_empty());
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
                Role::RadioGroup if self.radio_group_has_multiple_checked_children(node) => {
                    Some(RadioGroupWithMultipleCheckedChildren)
                }
                Role::Alert | Role::Status if node.state.live_region => Some(LiveAlertOrStatus),
                Role::Complementary | Role::ContentInfo | Role::Main if has_landmark => {
                    Some(NestedLandmark)
                }
                Role::Region if !has_label => Some(RegionNeedsLabel),
                Role::Slider | Role::ProgressBar => range_issue(node),
                _ => None,
            };
            if let Some(kind) = issue {
                reports.push(SemanticRoleIssue {
                    node: id,
                    role: node.role,
                    kind,
                });
            } else if let Some(index) = repeating_landmark
                && !has_label
            {
                unlabeled.push((
                    index,
                    SemanticRoleIssue {
                        node: id,
                        role: node.role,
                        kind: RepeatedLandmarkNeedsLabel,
                    },
                ));
            }
            let has_menu = has_menu || matches!(node.role, Role::Menu | Role::MenuBar);
            let has_landmark =
                has_landmark || repeating_landmark.is_some() || node.role == Role::Region;
            work.extend(
                node.children
                    .iter()
                    .rev()
                    .map(|&child| (child, Some(node.role), has_menu, has_landmark)),
            );
        }
        // Flutter requires a nonempty label when the role occurs again.
        // Its debug check doesn't compare the text of the two labels.
        reports.extend(
            unlabeled
                .into_iter()
                .filter_map(|(index, issue)| (counts[index] > 1).then_some(issue)),
        );
        reports
    }
    fn radio_group_has_multiple_checked_children(&self, group: &SemanticNode) -> bool {
        let mut seen = HashSet::from([group.id]);
        let mut work = group.children.clone();
        let mut checked = false;
        while let Some(id) = work.pop() {
            let Some(child) = self.node(id).filter(|_| seen.insert(id)) else {
                continue;
            };
            if child.role == Role::RadioGroup {
                // A nested group owns its own checked children.
                continue;
            }
            if child
                .state
                .annotation_flags
                .get(SemanticAnnotationFlag::MutuallyExclusiveGroup)
                == Some(true)
            {
                if child.state.checked.is_some_and(|state| state.is_checked()) {
                    if checked {
                        return true;
                    }
                    checked = true;
                }
            } else {
                work.extend(&child.children);
            }
        }
        false
    }
}

fn repeatable_landmark_index(role: Role) -> Option<usize> {
    match role {
        Role::Complementary => Some(0),
        Role::ContentInfo => Some(1),
        Role::Main => Some(2),
        Role::Navigation => Some(3),
        _ => None,
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
