use std::{collections::HashSet, fmt};

use crate::{Role, SemanticActionKind, SemanticNodeId, SemanticsTree};

/// The accessibility contract violated by a finished semantic node.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SemanticRoleIssueKind {
    TabWithoutSelectedState,
    EnabledTabWithoutTap,
    ListItemOutsideList,
    EmptyTabList,
    TabListWithNonTabChild,
    EmptyMenu,
    MenuItemOutsideMenu,
    LiveAlertOrStatus,
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
            Self::MenuItemOutsideMenu => "A menu item must be a child of a menu or a menu bar",
            Self::LiveAlertOrStatus => "An alert or status node cannot also be a live region",
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
                Role::MenuItem if !has_menu => Some(MenuItemOutsideMenu),
                Role::Alert | Role::Status if node.state.live_region => Some(LiveAlertOrStatus),
                _ => None,
            };
            if let Some(kind) = issue {
                reports.push(SemanticRoleIssue {
                    node: id,
                    role: node.role,
                    kind,
                });
            }
            let has_menu = has_menu || node.role == Role::Menu;
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
