//! Double-precision projective geometry for Flutter-compatible matrix utilities.
//!
//! The ordinary retained scene still uses the smaller affine [`crate::Transform`]
//! on its hot paths. These utilities do not imply perspective for every widget.

/// A 64-bit point for projective calculations without an early f32 rounding.
pub use kurbo::Point as Point64;
/// A rectangle represented by four f64 edges, not by origin and extent.
pub use kurbo::Rect as Rect64;

/// Flutter's cylindrical projection orientation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CylindricalOrientation {
    Horizontal,
    #[default]
    Vertical,
}

/// A column-major matrix operating on column vectors.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Matrix4([f64; 16]);

impl Matrix4 {
    pub const ZERO: Self = Self([0.; 16]);
    pub const IDENTITY: Self = Self([
        1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1.,
    ]);

    /// Constructs a matrix from column-major entries.
    #[must_use]
    pub const fn from_columns(values: [f64; 16]) -> Self {
        Self(values)
    }

    /// Returns the column-major entries, matching Flutter's storage order.
    #[must_use]
    pub const fn columns(&self) -> &[f64; 16] {
        &self.0
    }

    /// Orthographically projects the matrix's local z=0 plane into XY.
    /// A constant nonzero homogeneous denominator is affine even when a
    /// matrix rotates out of plane. Perspective that varies X/Y's
    /// denominator is not representable by the retained affine path.
    #[must_use]
    pub fn affine_xy_plane(self) -> Option<crate::Transform> {
        let [a, b, _, wx, c, d, _, wy, _, _, _, _, x, y, _, w] = self.0;
        if wx != 0.0 || wy != 0.0 || w == 0.0 || !w.is_finite() {
            return None;
        }
        let matrix = [a / w, b / w, c / w, d / w, x / w, y / w];
        matrix
            .iter()
            .all(|v| v.is_finite())
            .then(|| crate::Transform::from_coefficients(matrix))
    }

    #[must_use]
    pub const fn translation(x: f64, y: f64, z: f64) -> Self {
        let mut matrix = Self::IDENTITY;
        matrix.0[12] = x;
        matrix.0[13] = y;
        matrix.0[14] = z;
        matrix
    }

    #[must_use]
    pub const fn scale(x: f64, y: f64, z: f64) -> Self {
        Self([x, 0., 0., 0., 0., y, 0., 0., 0., 0., z, 0., 0., 0., 0., 1.])
    }

    #[must_use]
    pub fn rotation_x(angle: f64) -> Self {
        let (s, c) = angle.sin_cos();
        Self([1., 0., 0., 0., 0., c, s, 0., 0., -s, c, 0., 0., 0., 0., 1.])
    }

    #[must_use]
    pub fn rotation_y(angle: f64) -> Self {
        let (s, c) = angle.sin_cos();
        Self([c, 0., -s, 0., 0., 1., 0., 0., s, 0., c, 0., 0., 0., 0., 1.])
    }

    #[must_use]
    pub fn rotation_z(angle: f64) -> Self {
        let (s, c) = angle.sin_cos();
        Self([c, s, 0., 0., -s, c, 0., 0., 0., 0., 1., 0., 0., 0., 0., 1.])
    }

    /// Multiplies `self × rhs`, applying the right-hand geometry first.
    #[must_use]
    pub fn multiply(self, rhs: Self) -> Self {
        let mut result = Self::ZERO;
        let a = &self.0;
        for (output, b) in result.0.chunks_exact_mut(4).zip(rhs.0.chunks_exact(4)) {
            for (row, element) in output.iter_mut().enumerate() {
                *element =
                    a[row] * b[0] + a[4 + row] * b[1] + a[8 + row] * b[2] + a[12 + row] * b[3];
            }
        }
        result
    }

    /// Stores `self × rhs` in the right-hand operand, leaving `self` intact.
    pub fn multiply_in_place(self, rhs: &mut Self) {
        *rhs = self.multiply(*rhs);
    }

    /// Extracts the translation only for a pure 2D translation matrix.
    #[must_use]
    pub fn as_translation(self) -> Option<Point64> {
        let dx = self.0[12];
        let dy = self.0[13];
        let mut without_offset = self;
        without_offset.0[12] = 0.;
        without_offset.0[13] = 0.;
        (without_offset == Self::IDENTITY).then(|| Point64::new(dx, dy))
    }

    /// Extracts the scale only for a uniform scale in x and y with z intact.
    #[must_use]
    pub fn as_scale(self) -> Option<f64> {
        let scale = self.0[0];
        (self == Self::scale(scale, scale, 1.)).then_some(scale)
    }

    /// Collapses the x/y geometry to the supplied point without altering z.
    #[must_use]
    pub fn force_to_point(point: Point64) -> Self {
        Self([
            0., 0., 0., 0., 0., 0., 0., 0., 0., 0., 1., 0., point.x, point.y, 0., 1.,
        ])
    }

    /// Projects an object on a vertical cylinder with perspective `0.001`.
    #[must_use]
    pub fn cylindrical_projection(radius: f64, angle: f64) -> Self {
        Self::cylindrical_projection_with(radius, angle, 0.001, CylindricalOrientation::Vertical)
    }

    /// Projects a plane on a cylinder while preserving its scale at angle zero.
    ///
    /// # Panics
    ///
    /// Panics unless perspective is a value between zero and one.
    #[must_use]
    pub fn cylindrical_projection_with(
        radius: f64,
        angle: f64,
        perspective: f64,
        orientation: CylindricalOrientation,
    ) -> Self {
        assert!(
            (0. ..=1.).contains(&perspective),
            "perspective must be between zero and one"
        );
        let mut view = Self::translation(0., 0., -radius);
        view.0[11] = -perspective;
        view.0[15] = 1. + perspective * radius;
        let rotation = match orientation {
            CylindricalOrientation::Horizontal => Self::rotation_y(angle),
            CylindricalOrientation::Vertical => Self::rotation_x(angle),
        };
        view.multiply(rotation.multiply(Self::translation(0., 0., radius)))
    }

    /// Transforms a point on z = 0 and divides by the homogeneous w
    /// coordinate. Points on the projection pole can produce nonfinite values.
    #[must_use]
    pub fn transform_point(self, point: Point64) -> Point64 {
        let m = self.0;
        let x = m[0] * point.x + m[4] * point.y + m[12];
        let y = m[1] * point.x + m[5] * point.y + m[13];
        let w = m[3] * point.x + m[7] * point.y + m[15];
        if w == 1. {
            Point64::new(x, y)
        } else {
            Point64::new(x / w, y / w)
        }
    }

    /// Bounds the transformed corners. Uses the edges directly so subtracting
    /// two very large finite edges cannot overflow before the transform.
    #[must_use]
    pub fn transform_rect(self, rect: Rect64) -> Rect64 {
        if rect.x0 <= rect.x1
            && rect.y0 <= rect.y1
            && let Some(offset) = self.as_translation()
        {
            return Rect64::new(
                rect.x0 + offset.x,
                rect.y0 + offset.y,
                rect.x1 + offset.x,
                rect.y1 + offset.y,
            );
        }
        let points = [
            self.transform_point(Point64::new(rect.x0, rect.y0)),
            self.transform_point(Point64::new(rect.x1, rect.y0)),
            self.transform_point(Point64::new(rect.x0, rect.y1)),
            self.transform_point(Point64::new(rect.x1, rect.y1)),
        ];
        // Unlike f64::min/max, the reference propagates a NaN from any
        // point on an axis rather than silently discarding that corner.
        let bounds = |values: [f64; 4]| {
            if values.iter().any(|value| value.is_nan()) {
                (f64::NAN, f64::NAN)
            } else {
                (
                    values.into_iter().reduce(f64::min).unwrap(),
                    values.into_iter().reduce(f64::max).unwrap(),
                )
            }
        };
        let (left, right) = bounds(points.map(|point| point.x));
        let (top, bottom) = bounds(points.map(|point| point.y));
        Rect64::new(left, top, right, bottom)
    }
}

impl Default for Matrix4 {
    fn default() -> Self {
        Self::IDENTITY
    }
}
