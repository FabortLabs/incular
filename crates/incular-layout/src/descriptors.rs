use crate::{
    Alignment, Axis, Constraints, CrossAxisAlignment, MainAxisAlignment, MainAxisSize,
    TextDirection, VerticalDirection, WrapAlignment, WrapCrossAlignment,
};

/// Shared options used by a pure layout pass. It intentionally has no child
/// or widget references; callers provide measured children to an algorithm.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct LayoutOptions {
    pub text_direction: TextDirection,
    pub vertical_direction: VerticalDirection,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Flex {
    pub direction: Axis,
    pub main_axis_size: MainAxisSize,
    pub main_axis_alignment: MainAxisAlignment,
    pub cross_axis_alignment: CrossAxisAlignment,
    pub text_direction: TextDirection,
    pub vertical_direction: VerticalDirection,
    pub spacing: f32,
}

impl Default for Flex {
    fn default() -> Self {
        Self::new(Axis::Horizontal)
    }
}

impl Flex {
    #[must_use]
    pub const fn new(direction: Axis) -> Self {
        Self {
            direction,
            main_axis_size: MainAxisSize::Max,
            main_axis_alignment: MainAxisAlignment::Start,
            cross_axis_alignment: CrossAxisAlignment::Center,
            text_direction: TextDirection::Ltr,
            vertical_direction: VerticalDirection::Down,
            spacing: 0.0,
        }
    }

    #[must_use]
    pub const fn row() -> Self {
        Self::new(Axis::Horizontal)
    }

    #[must_use]
    pub const fn column() -> Self {
        Self::new(Axis::Vertical)
    }

    #[must_use]
    pub const fn with_main_axis_size(mut self, value: MainAxisSize) -> Self {
        self.main_axis_size = value;
        self
    }

    #[must_use]
    pub const fn with_main_axis_alignment(mut self, value: MainAxisAlignment) -> Self {
        self.main_axis_alignment = value;
        self
    }

    #[must_use]
    pub const fn with_cross_axis_alignment(mut self, value: CrossAxisAlignment) -> Self {
        self.cross_axis_alignment = value;
        self
    }

    #[must_use]
    pub const fn with_text_direction(mut self, value: TextDirection) -> Self {
        self.text_direction = value;
        self
    }

    #[must_use]
    pub const fn with_vertical_direction(mut self, value: VerticalDirection) -> Self {
        self.vertical_direction = value;
        self
    }

    #[must_use]
    pub const fn with_spacing(mut self, value: f32) -> Self {
        self.spacing = value;
        self
    }
}

/// Flutter-compatible row descriptor. It carries policy only; children are
/// passed to [`crate::layout_flex`] as measured values.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Row {
    pub flex: Flex,
}

impl Default for Row {
    fn default() -> Self {
        Self::new()
    }
}

impl Row {
    #[must_use]
    pub const fn new() -> Self {
        Self { flex: Flex::row() }
    }

    #[must_use]
    pub const fn with_flex(mut self, flex: Flex) -> Self {
        self.flex = Flex {
            direction: Axis::Horizontal,
            ..flex
        };
        self
    }

    #[must_use]
    pub const fn policy(self) -> Flex {
        self.flex
    }
}

/// Flutter-compatible column descriptor. It carries policy only; children are
/// passed to [`crate::layout_flex`] as measured values.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Column {
    pub flex: Flex,
}

impl Default for Column {
    fn default() -> Self {
        Self::new()
    }
}

impl Column {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            flex: Flex::column(),
        }
    }

    #[must_use]
    pub const fn with_flex(mut self, flex: Flex) -> Self {
        self.flex = Flex {
            direction: Axis::Vertical,
            ..flex
        };
        self
    }

    #[must_use]
    pub const fn policy(self) -> Flex {
        self.flex
    }
}

/// Stack layout policy. Alignment factors are absolute, so the policy
/// carries no text direction; directional placement is the caller's
/// concern via the alignment it resolves before layout.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Stack {
    pub alignment: Alignment,
    pub fit: StackFit,
    pub clip_behavior: Clip,
}

impl Default for Stack {
    fn default() -> Self {
        Self::new()
    }
}

impl Stack {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            alignment: Alignment::TOP_LEFT,
            fit: StackFit::Loose,
            clip_behavior: Clip::HardEdge,
        }
    }

    #[must_use]
    pub const fn with_alignment(mut self, value: Alignment) -> Self {
        self.alignment = value;
        self
    }

    #[must_use]
    pub const fn with_fit(mut self, value: StackFit) -> Self {
        self.fit = value;
        self
    }
}

pub use incular_config::{Clip, StackFit};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Wrap {
    pub direction: Axis,
    pub spacing: f32,
    pub run_spacing: f32,
    pub alignment: WrapAlignment,
    pub run_alignment: WrapAlignment,
    pub cross_axis_alignment: WrapCrossAlignment,
    pub text_direction: TextDirection,
    pub vertical_direction: VerticalDirection,
}

impl Default for Wrap {
    fn default() -> Self {
        Self::new(Axis::Horizontal)
    }
}

impl Wrap {
    #[must_use]
    pub const fn new(direction: Axis) -> Self {
        Self {
            direction,
            spacing: 0.0,
            run_spacing: 0.0,
            alignment: WrapAlignment::Start,
            run_alignment: WrapAlignment::Start,
            cross_axis_alignment: WrapCrossAlignment::Start,
            text_direction: TextDirection::Ltr,
            vertical_direction: VerticalDirection::Down,
        }
    }
}

/// How one table column's width is chosen, following Flutter's
/// `TableColumnWidth` family.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TableColumnWidth {
    /// A fixed width in logical pixels (`FixedColumnWidth`).
    Fixed(f32),
    /// A fraction of the table's maximum width (`FractionColumnWidth`).
    Fraction(f32),
    /// The widest cell in the column (`IntrinsicColumnWidth`).
    Intrinsic,
    /// A share of the space left after the other columns, proportional to
    /// the flex factor (`FlexColumnWidth`). Non-positive factors behave as
    /// [`TableColumnWidth::Intrinsic`].
    Flex(f32),
}

impl Default for TableColumnWidth {
    /// Flutter's default table column width: `FlexColumnWidth()`.
    fn default() -> Self {
        Self::Flex(1.0)
    }
}

/// Vertical placement of a cell within its row (`TableCellVerticalAlignment`).
///
/// `Fill`, `Baseline` and `IntrinsicHeight` are not supported yet; cells use
/// one of the three positional values below.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TableCellVerticalAlignment {
    /// Cells sit at the top of the row (Flutter's default).
    #[default]
    Top,
    /// Cells are centered vertically in the row.
    Middle,
    /// Cells sit at the bottom of the row.
    Bottom,
}

impl TableCellVerticalAlignment {
    pub(crate) const fn factor(self) -> f32 {
        match self {
            Self::Top => 0.0,
            Self::Middle => 0.5,
            Self::Bottom => 1.0,
        }
    }
}

/// Table layout policy: a fixed column count with per-column width specs,
/// row heights from the tallest cell, and a table-wide vertical alignment.
///
/// Column widths follow Flutter's `Table._computeColumnWidths`: flex columns
/// share the space left after the other columns when the table has a bounded
/// width. Cells are laid out within their slot at the top, middle or bottom.
#[derive(Clone, Debug, PartialEq)]
pub struct Table {
    pub columns: usize,
    pub column_spacing: f32,
    pub row_spacing: f32,
    /// Per-column overrides as `(column, width)`. The last entry for a column wins.
    pub column_widths: Vec<(usize, TableColumnWidth)>,
    /// Width for columns without an override (Flutter default: flex 1.0).
    pub default_column_width: TableColumnWidth,
    pub default_vertical_alignment: TableCellVerticalAlignment,
}

impl Table {
    /// The width spec in effect for one column.
    #[must_use]
    pub fn column_width(&self, column: usize) -> TableColumnWidth {
        self.column_widths
            .iter()
            .rev()
            .find(|(index, _)| *index == column)
            .map_or(self.default_column_width, |(_, width)| *width)
    }
}

impl Default for Table {
    fn default() -> Self {
        Self::new(1)
    }
}

impl Table {
    #[must_use]
    pub const fn new(columns: usize) -> Self {
        Self {
            columns,
            column_spacing: 0.0,
            row_spacing: 0.0,
            column_widths: Vec::new(),
            default_column_width: TableColumnWidth::Flex(1.0),
            default_vertical_alignment: TableCellVerticalAlignment::Top,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Align {
    pub alignment: Alignment,
    pub width_factor: Option<f32>,
    pub height_factor: Option<f32>,
}

impl Align {
    #[must_use]
    pub const fn new(alignment: Alignment) -> Self {
        Self {
            alignment,
            width_factor: None,
            height_factor: None,
        }
    }

    #[must_use]
    pub const fn with_width_factor(mut self, value: Option<f32>) -> Self {
        self.width_factor = value;
        self
    }

    #[must_use]
    pub const fn with_height_factor(mut self, value: Option<f32>) -> Self {
        self.height_factor = value;
        self
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Center {
    pub width_factor: Option<f32>,
    pub height_factor: Option<f32>,
}

impl Center {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            width_factor: None,
            height_factor: None,
        }
    }

    #[must_use]
    pub const fn alignment(self) -> Align {
        Align {
            alignment: Alignment::CENTER,
            width_factor: self.width_factor,
            height_factor: self.height_factor,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Padding {
    pub padding: crate::EdgeInsets,
}

impl Default for Padding {
    fn default() -> Self {
        Self::new(crate::EdgeInsets::ZERO)
    }
}

impl Padding {
    #[must_use]
    pub const fn new(padding: crate::EdgeInsets) -> Self {
        Self { padding }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SizedBox {
    pub width: Option<f32>,
    pub height: Option<f32>,
}

impl SizedBox {
    #[must_use]
    pub const fn new(width: Option<f32>, height: Option<f32>) -> Self {
        Self { width, height }
    }

    #[must_use]
    pub const fn from_size(size: crate::Size) -> Self {
        Self::new(Some(size.width), Some(size.height))
    }

    #[must_use]
    pub const fn square(value: f32) -> Self {
        Self::new(Some(value), Some(value))
    }

    #[must_use]
    pub const fn shrink() -> Self {
        Self::new(Some(0.0), Some(0.0))
    }

    #[must_use]
    pub const fn expand() -> Self {
        Self::new(None, None)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ConstrainedBox {
    pub constraints: Constraints,
}

impl ConstrainedBox {
    #[must_use]
    pub const fn new(constraints: Constraints) -> Self {
        Self { constraints }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UnconstrainedBox {
    pub alignment: Alignment,
    pub constrained_axis: Option<Axis>,
}

impl Default for UnconstrainedBox {
    fn default() -> Self {
        Self::new()
    }
}

impl UnconstrainedBox {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            alignment: Alignment::CENTER,
            constrained_axis: None,
        }
    }

    #[must_use]
    pub const fn with_alignment(mut self, value: Alignment) -> Self {
        self.alignment = value;
        self
    }

    #[must_use]
    pub const fn with_constrained_axis(mut self, value: Option<Axis>) -> Self {
        self.constrained_axis = value;
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AspectRatio {
    pub aspect_ratio: f32,
}

impl AspectRatio {
    #[must_use]
    pub const fn new(aspect_ratio: f32) -> Self {
        Self { aspect_ratio }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum BaselineType {
    #[default]
    Alphabetic,
    Ideographic,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Baseline {
    pub baseline: f32,
    pub baseline_type: BaselineType,
}

impl Baseline {
    #[must_use]
    pub const fn new(baseline: f32, baseline_type: BaselineType) -> Self {
        Self {
            baseline,
            baseline_type,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FractionallySizedBox {
    pub width_factor: Option<f32>,
    pub height_factor: Option<f32>,
    pub alignment: Alignment,
}

impl Default for FractionallySizedBox {
    fn default() -> Self {
        Self::new(None, None)
    }
}

impl FractionallySizedBox {
    #[must_use]
    pub const fn new(width_factor: Option<f32>, height_factor: Option<f32>) -> Self {
        Self {
            width_factor,
            height_factor,
            alignment: Alignment::CENTER,
        }
    }

    #[must_use]
    pub const fn with_alignment(mut self, value: Alignment) -> Self {
        self.alignment = value;
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Positioned {
    pub left: Option<f32>,
    pub top: Option<f32>,
    pub right: Option<f32>,
    pub bottom: Option<f32>,
    pub width: Option<f32>,
    pub height: Option<f32>,
}

impl Default for Positioned {
    fn default() -> Self {
        Self::new()
    }
}

impl Positioned {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            left: None,
            top: None,
            right: None,
            bottom: None,
            width: None,
            height: None,
        }
    }

    /// An empty set of insets and extents participates in non-positioned
    /// Stack sizing, like Flutter's `StackParentData.isPositioned`.
    #[must_use]
    pub const fn is_positioned(self) -> bool {
        self.left.is_some()
            || self.top.is_some()
            || self.right.is_some()
            || self.bottom.is_some()
            || self.width.is_some()
            || self.height.is_some()
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Visibility {
    pub visible: bool,
    pub maintain_state: bool,
    pub maintain_animation: bool,
    pub maintain_size: bool,
    pub maintain_semantics: bool,
    pub maintain_interactivity: bool,
}

impl Visibility {
    #[must_use]
    pub const fn new(visible: bool) -> Self {
        Self {
            visible,
            maintain_state: false,
            maintain_animation: false,
            maintain_size: false,
            maintain_semantics: false,
            maintain_interactivity: false,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Offstage {
    pub offstage: bool,
}

impl Offstage {
    #[must_use]
    pub const fn new(offstage: bool) -> Self {
        Self { offstage }
    }
}
