//! Nonfatal, debug-only reports for painted Flex overflow.

use std::fmt;

use incular_config::Axis;

use super::{RenderObjectId, WidgetTree};

/// A debug summary for one painted RenderFlex that exceeded its main axis.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FlexOverflowDiagnostic {
    pub render: RenderObjectId,
    pub axis: Axis,
    pub pixels: f64,
}

impl FlexOverflowDiagnostic {
    /// The edge used by Flutter's simulated overflow rectangle.
    #[must_use]
    pub const fn edge(&self) -> &'static str {
        match self.axis {
            Axis::Horizontal => "right",
            Axis::Vertical => "bottom",
        }
    }
}

impl fmt::Display for FlexOverflowDiagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "A RenderFlex overflowed by {} pixels on the {}.",
            overflow_pixels_text(self.pixels),
            self.edge(),
        )
    }
}

impl std::error::Error for FlexOverflowDiagnostic {}

/// Debug summary of the physical edges exceeded by a constraint transformation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ConstraintOverflowDiagnostic {
    pub render: RenderObjectId,
    pub left: f64,
    pub top: f64,
    pub bottom: f64,
    pub right: f64,
}

impl ConstraintOverflowDiagnostic {
    /// Overflow is a diagnostic summary, not a panic or a layout error.
    #[must_use]
    pub const fn level(&self) -> incular_core::DiagnosticLevel {
        incular_core::DiagnosticLevel::Summary
    }
}

impl fmt::Display for ConstraintOverflowDiagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let edges = [
            ("left", self.left),
            ("top", self.top),
            ("bottom", self.bottom),
            ("right", self.right),
        ];
        let clauses: Vec<_> = edges
            .into_iter()
            .filter(|(_, pixels)| *pixels > 0.)
            .map(|(side, pixels)| format!("{} pixels on the {side}", overflow_pixels_text(pixels)))
            .collect();
        let summary = match clauses.as_slice() {
            [] => String::new(),
            [only] => only.clone(),
            [first, second] => format!("{first} and {second}"),
            [prefix @ .., last] => format!("{}, and {last}", prefix.join(", ")),
        };
        write!(
            f,
            "A RenderConstraintsTransformBox overflowed by {summary}."
        )
    }
}

impl std::error::Error for ConstraintOverflowDiagnostic {}

pub(super) fn overflow_pixels_text(pixels: f64) -> String {
    if pixels > 10. {
        format!("{:.0}", pixels.round())
    } else if pixels > 1. {
        format!("{:.1}", (pixels * 10.).round() / 10.)
    } else if pixels > 0. {
        // Flutter uses three significant digits at or below one pixel.
        let places = 2 - pixels.log10().floor() as i32;
        let factor = 10_f64.powi(places);
        let rounded = (pixels * factor).round() / factor;
        let scientific = format!("{rounded:.2e}");
        let exponent: i32 = scientific
            .rsplit_once('e')
            .and_then(|(_, exponent)| exponent.parse().ok())
            .unwrap_or_default();
        if exponent < -6 {
            scientific
        } else {
            format!(
                "{rounded:.precision$}",
                precision = (2 - exponent).max(0) as usize
            )
        }
    } else {
        pixels.to_string()
    }
}

#[cfg(debug_assertions)]
enum ReportState {
    Pending(FlexOverflowDiagnostic),
    Reported(Option<FlexOverflowDiagnostic>),
}

#[cfg(debug_assertions)]
#[derive(Default)]
pub(super) struct FlexOverflowDiagnostics {
    states: super::FastHashMap<RenderObjectId, ReportState>,
    reports: std::collections::VecDeque<FlexOverflowDiagnostic>,
    constraint_states: super::FastHashSet<RenderObjectId>,
    constraint_reports: std::collections::VecDeque<ConstraintOverflowDiagnostic>,
}

impl WidgetTree {
    /// Drains recent nonfatal Flex overflow reports in paint order. Release
    /// builds produce no reports. Debug builds retain up to 256 undrained
    /// reports and report a mounted render object only on its first overflow.
    pub fn take_flex_overflow_diagnostics(&mut self) -> Vec<FlexOverflowDiagnostic> {
        #[cfg(debug_assertions)]
        if let Some(diagnostics) = &mut self.flex_overflow_diagnostics {
            return diagnostics.reports.drain(..).collect();
        }
        Vec::new()
    }

    #[cfg(debug_assertions)]
    pub(super) fn note_flex_overflow_layout(
        &mut self,
        render: RenderObjectId,
        axis: Axis,
        pixels: f64,
    ) {
        if pixels.is_finite() && pixels > 1e-10 {
            let diagnostics = self
                .flex_overflow_diagnostics
                .get_or_insert_with(Default::default);
            let diagnostic = FlexOverflowDiagnostic {
                render,
                axis,
                pixels,
            };
            let state = diagnostics
                .states
                .entry(render)
                .or_insert(ReportState::Pending(diagnostic));
            *state = match state {
                ReportState::Pending(_) => ReportState::Pending(diagnostic),
                ReportState::Reported(_) => ReportState::Reported(Some(diagnostic)),
            };
        } else if let Some(diagnostics) = &mut self.flex_overflow_diagnostics {
            match diagnostics.states.get_mut(&render) {
                Some(ReportState::Reported(last)) => *last = None,
                Some(ReportState::Pending(_)) => {
                    diagnostics.states.remove(&render);
                }
                None => {}
            }
        }
    }

    #[cfg(debug_assertions)]
    pub(super) fn active_flex_overflow(
        &self,
        render: RenderObjectId,
    ) -> Option<FlexOverflowDiagnostic> {
        match self
            .flex_overflow_diagnostics
            .as_ref()?
            .states
            .get(&render)?
        {
            ReportState::Pending(report) => Some(*report),
            ReportState::Reported(report) => *report,
        }
    }

    #[cfg(debug_assertions)]
    pub(super) fn publish_flex_overflow_on_paint(&mut self, render: RenderObjectId) {
        let Some(diagnostics) = &mut self.flex_overflow_diagnostics else {
            return;
        };
        let Some(state) = diagnostics.states.get_mut(&render) else {
            return;
        };
        if let ReportState::Pending(report) = *state {
            *state = ReportState::Reported(Some(report));
            if diagnostics.reports.len() == 256 {
                diagnostics.reports.pop_front();
            }
            diagnostics.reports.push_back(report);
        }
    }

    #[cfg(debug_assertions)]
    pub(super) fn forget_flex_overflow(&mut self, render: RenderObjectId) {
        if let Some(diagnostics) = &mut self.flex_overflow_diagnostics {
            diagnostics.states.remove(&render);
            diagnostics.constraint_states.remove(&render);
        }
    }

    /// Drains debug-only reports for the first visible overflow of an
    /// unconstrained or constraints-transform render object.
    pub fn take_constraint_overflow_diagnostics(&mut self) -> Vec<ConstraintOverflowDiagnostic> {
        #[cfg(debug_assertions)]
        if let Some(diagnostics) = &mut self.flex_overflow_diagnostics {
            return diagnostics.constraint_reports.drain(..).collect();
        }
        Vec::new()
    }

    #[cfg(debug_assertions)]
    pub(super) fn publish_constraint_overflow(&mut self, report: ConstraintOverflowDiagnostic) {
        let diagnostics = self
            .flex_overflow_diagnostics
            .get_or_insert_with(Default::default);
        if diagnostics.constraint_states.insert(report.render) {
            if diagnostics.constraint_reports.len() == 256 {
                diagnostics.constraint_reports.pop_front();
            }
            diagnostics.constraint_reports.push_back(report);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::overflow_pixels_text;

    #[test]
    fn overflow_summary_uses_flutter_precision_thresholds() {
        for (pixels, expected) in [
            (125., "125"),
            (12.5, "13"),
            (10., "10.0"),
            (1.125, "1.1"),
            (1., "1.00"),
            (0.3125, "0.313"),
            (0.5, "0.500"),
            (0.000_000_25, "2.50e-7"),
        ] {
            assert_eq!(overflow_pixels_text(pixels), expected);
        }
    }
}
