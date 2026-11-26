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

fn overflow_pixels_text(pixels: f64) -> String {
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
    Reported,
}

#[cfg(debug_assertions)]
#[derive(Default)]
pub(super) struct FlexOverflowDiagnostics {
    states: super::FastHashMap<RenderObjectId, ReportState>,
    reports: std::collections::VecDeque<FlexOverflowDiagnostic>,
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
            let state = diagnostics
                .states
                .entry(render)
                .or_insert(ReportState::Pending(FlexOverflowDiagnostic {
                    render,
                    axis,
                    pixels,
                }));
            if let ReportState::Pending(diagnostic) = state {
                diagnostic.pixels = pixels;
            }
        } else if let Some(diagnostics) = &mut self.flex_overflow_diagnostics
            && matches!(
                diagnostics.states.get(&render),
                Some(ReportState::Pending(_))
            )
        {
            diagnostics.states.remove(&render);
        }
    }

    #[cfg(debug_assertions)]
    pub(super) fn publish_flex_overflow_on_paint(&mut self, render: RenderObjectId) {
        let Some(diagnostics) = &mut self.flex_overflow_diagnostics else {
            return;
        };
        let Some(state @ ReportState::Pending(_)) = diagnostics.states.get_mut(&render) else {
            return;
        };
        if let ReportState::Pending(report) = std::mem::replace(state, ReportState::Reported) {
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
