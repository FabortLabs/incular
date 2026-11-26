//! Weighted polynomial fitting used by Flutter-style velocity estimation.

use incular_core::debug_double::{format_double_fixed, format_double_precision};
use std::fmt;

const PRECISION_ERROR_TOLERANCE: f64 = 1e-10;

/// A polynomial whose coefficient at index `i` multiplies `x^i`.
#[derive(Clone, Debug, PartialEq)]
pub struct PolynomialFit {
    pub coefficients: Vec<f64>,
    /// Fraction of weighted variance explained by the fit.
    pub confidence: f64,
}

impl PolynomialFit {
    /// Allocates the `degree + 1` coefficients, initially zero.
    #[must_use]
    pub fn new(degree: usize) -> Self {
        Self {
            coefficients: vec![0.; degree.checked_add(1).expect("polynomial degree overflow")],
            confidence: 0.,
        }
    }
}

impl fmt::Display for PolynomialFit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PolynomialFit([")?;
        for (index, &value) in self.coefficients.iter().enumerate() {
            if index != 0 {
                write!(f, ", ")?;
            }
            write!(f, "{}", format_double_precision(value, 3))?;
        }
        write!(
            f,
            "], confidence: {})",
            format_double_fixed(self.confidence, 3)
        )
    }
}

/// Fits a polynomial to equally sized slices of positions and weights.
pub struct LeastSquaresSolver<'a> {
    pub x: &'a [f64],
    pub y: &'a [f64],
    pub w: &'a [f64],
}

impl<'a> LeastSquaresSolver<'a> {
    /// # Panics
    ///
    /// Panics if the slices have different lengths.
    #[must_use]
    pub fn new(x: &'a [f64], y: &'a [f64], w: &'a [f64]) -> Self {
        assert_eq!(x.len(), y.len(), "one y coordinate is required for each x");
        assert_eq!(y.len(), w.len(), "one weight is required for each point");
        Self { x, y, w }
    }

    /// Returns no fit for insufficient data or a rank-deficient design.
    #[must_use]
    pub fn solve(&self, degree: usize) -> Option<PolynomialFit> {
        let width = degree.checked_add(1)?;
        let count = self.x.len();
        if width > count || self.y.len() != count || self.w.len() != count {
            return None;
        }
        let matrix_len = width.checked_mul(count)?;
        let scratch_len = matrix_len
            .checked_mul(2)?
            .checked_add(width.checked_mul(width)?)?;
        let mut storage = vec![0.; scratch_len];
        let (design, rest) = storage.split_at_mut(matrix_len);
        let (basis, triangular) = rest.split_at_mut(matrix_len);
        let mut fit = PolynomialFit::new(degree);
        let factor = FitScratch {
            design,
            basis,
            triangular,
            stride: count,
        }
        .factor(self.x, Some(self.w), width)?;
        factor.solve_response(self.y, &mut fit.coefficients);
        let mean = self.y.iter().sum::<f64>() / count as f64;
        let (mut squared_error, mut squared_total) = (0., 0.);
        for ((&x, &y), &w) in self.x.iter().zip(self.y).zip(self.w) {
            let mut term = 1.;
            let mut error = y - fit.coefficients[0];
            for coefficient in &fit.coefficients[1..] {
                term *= x;
                error -= term * coefficient;
            }
            squared_error += w * w * error * error;
            let variation = y - mean;
            squared_total += w * w * variation * variation;
        }
        fit.confidence = if squared_total <= PRECISION_ERROR_TOLERANCE {
            1.
        } else {
            1. - squared_error / squared_total
        };
        Some(fit)
    }
}

pub(crate) struct FitScratch<'a> {
    pub(crate) design: &'a mut [f64],
    pub(crate) basis: &'a mut [f64],
    pub(crate) triangular: &'a mut [f64],
    pub(crate) stride: usize,
}

impl<'a> FitScratch<'a> {
    /// Factors the weighted design once. Several responses with the same
    /// independent variable (for example both pointer axes) can reuse it.
    #[inline]
    pub(crate) fn factor(
        self,
        x: &[f64],
        weights: Option<&'a [f64]>,
        width: usize,
    ) -> Option<FactoredFit<'a>> {
        let count = x.len();
        let Self {
            design,
            basis,
            triangular,
            stride,
        } = self;
        if let Some(weights) = weights {
            design[..count].copy_from_slice(weights);
        } else {
            design[..count].fill(1.);
        }
        for column in 1..width {
            let (prior, next) = design.split_at_mut(column * stride);
            let prior = &prior[(column - 1) * stride..][..count];
            for ((element, &prior), &x) in next[..count].iter_mut().zip(prior).zip(x) {
                *element = prior * x;
            }
        }

        for column in 0..width {
            let offset = column * stride;
            let (prior, next) = basis.split_at_mut(offset);
            let current = &mut next[..count];
            current.copy_from_slice(&design[offset..][..count]);
            for prior in prior.chunks_exact(stride) {
                let prior = &prior[..count];
                let projection = dot(current, prior);
                for (entry, &other) in current.iter_mut().zip(prior) {
                    *entry -= projection * other;
                }
            }
            let norm = dot(current, current).sqrt();
            if norm < PRECISION_ERROR_TOLERANCE {
                return None;
            }
            let inverse_norm = 1. / norm;
            for entry in current.iter_mut() {
                *entry *= inverse_norm;
            }
            for next in column..width {
                triangular[column * width + next] = dot(current, &design[next * stride..][..count]);
            }
        }

        Some(FactoredFit {
            basis,
            triangular,
            stride,
            count,
            width,
            weights,
        })
    }
}

pub(crate) struct FactoredFit<'a> {
    basis: &'a [f64],
    triangular: &'a [f64],
    stride: usize,
    count: usize,
    width: usize,
    weights: Option<&'a [f64]>,
}

impl FactoredFit<'_> {
    #[inline]
    pub(crate) fn solve_response(&self, y: &[f64], coefficients: &mut [f64]) {
        let Self {
            basis,
            triangular,
            stride,
            count,
            width,
            weights,
        } = *self;
        debug_assert_eq!(count, y.len());
        debug_assert_eq!(width, coefficients.len());
        for column in (0..width).rev() {
            let basis = &basis[column * stride..][..count];
            let mut coefficient = if let Some(weights) = weights {
                basis
                    .iter()
                    .zip(y)
                    .zip(weights)
                    .fold(0., |sum, ((q, y), w)| sum + q * (y * w))
            } else {
                dot(basis, y)
            };
            for next in ((column + 1)..width).rev() {
                coefficient -= triangular[column * width + next] * coefficients[next];
            }
            coefficients[column] = coefficient / triangular[column * width + column];
        }
    }
}

fn dot(left: &[f64], right: &[f64]) -> f64 {
    left.iter().zip(right).fold(0., |sum, (a, b)| sum + a * b)
}
