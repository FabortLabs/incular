//! Declaration mistakes a motion site can make, reported once per site in
//! debug builds and listed by [`Motion::lints`](super::Motion::lints).

use std::{cell::RefCell, collections::HashSet, fmt};

use super::{
    Motion,
    cond::{Cond, Interaction},
    pose::Prop,
};

/// A likely mistake in a motion declaration.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MotionLint {
    /// `narrow` always holds while `wide` holds, but `wide` is declared
    /// later, so its pose wins for these properties and the `narrow` pose
    /// never shows them (for example `pressed` before `hovered`).
    Shadowed {
        narrow: Interaction,
        wide: Interaction,
        props: Vec<Prop>,
    },
    /// `layout()` (or `layout_position()`) on an unkeyed child: siblings
    /// that reorder are matched by position, so the wrong element animates.
    UnkeyedLayout,
}

impl fmt::Display for MotionLint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Shadowed {
                narrow,
                wide,
                props,
            } => write!(
                f,
                "motion: `{}` is declared before `{}`, which also holds while \
                 {} does, so the later pose wins for {:?}; declare `{}` last",
                name(*narrow),
                name(*wide),
                name(*narrow),
                props,
                name(*narrow),
            ),
            Self::UnkeyedLayout => write!(
                f,
                "motion: `layout()` on an unkeyed child; key the child \
                 (`.with_key(..)` before `.motion(..)`) so reordered siblings \
                 keep their identity"
            ),
        }
    }
}

pub(crate) fn name(interaction: Interaction) -> &'static str {
    match interaction {
        Interaction::Hovered => "hovered",
        Interaction::Pressed => "pressed",
        Interaction::Focused => "focused",
        Interaction::FocusVisible => "focus_visible",
        Interaction::Dragging => "dragging",
        Interaction::InView => "in_view",
        Interaction::DropOver => "drop_over",
        Interaction::DropActive => "drop_active",
    }
}

/// `(narrow, wide)`: `narrow` implies `wide` in practice.
const IMPLIES: [(Interaction, Interaction); 4] = [
    (Interaction::Pressed, Interaction::Hovered),
    (Interaction::FocusVisible, Interaction::Focused),
    (Interaction::Dragging, Interaction::Pressed),
    (Interaction::Dragging, Interaction::Hovered),
];

pub(crate) fn lints(motion: &Motion, child_keyed: bool) -> Vec<MotionLint> {
    let mut found = Vec::new();
    for (narrow, wide) in IMPLIES {
        let Some(at) = motion
            .states
            .iter()
            .position(|(cond, _)| matches!(cond, Cond::State(s) if *s == narrow))
        else {
            continue;
        };
        let narrow_pose = &motion.states[at].1;
        let props: Vec<Prop> = motion.states[at + 1..]
            .iter()
            .filter(|(cond, _)| matches!(cond, Cond::State(s) if *s == wide))
            .flat_map(|(_, pose)| pose.props().filter(|prop| narrow_pose.sets(*prop)))
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect();
        if !props.is_empty() {
            found.push(MotionLint::Shadowed {
                narrow,
                wide,
                props,
            });
        }
    }
    if motion.layout.is_some() && !child_keyed {
        found.push(MotionLint::UnkeyedLayout);
    }
    found
}

thread_local! {
    static REPORTED: RefCell<HashSet<String>> = RefCell::new(HashSet::new());
}

/// Prints each distinct lint once per thread in debug builds.
pub(crate) fn report(found: &[MotionLint]) {
    if !cfg!(debug_assertions) {
        return;
    }
    for lint in found {
        let message = lint.to_string();
        let fresh = REPORTED.with(|reported| reported.borrow_mut().insert(message.clone()));
        if fresh {
            eprintln!("warning: {message}");
        }
    }
}
