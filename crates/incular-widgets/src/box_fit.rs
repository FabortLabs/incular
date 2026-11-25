//! Flutter's renderer-independent source and destination sizing for box fits.

use incular_core::Size;

use crate::BoxFit;

/// The visible source size and the size it occupies after fitting.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FittedSizes {
    /// The portion of the original content to draw, before scaling.
    pub source: Size,
    /// The space occupied by that content in the destination.
    pub destination: Size,
}

impl FittedSizes {
    #[must_use]
    pub const fn new(source: Size, destination: Size) -> Self {
        Self {
            source,
            destination,
        }
    }
}

/// Fits content inside a box without choosing its alignment.
///
/// Both sizes are empty if an input extent is nonpositive or nonfinite.
/// `FitWidth` and `FitHeight` crop the source when the other dimension
/// would overflow. `None` crops without scaling.
#[must_use]
pub fn apply_box_fit(fit: BoxFit, input: Size, output: Size) -> FittedSizes {
    for extent in [input.width, input.height, output.width, output.height] {
        if extent <= 0.0 || !extent.is_finite() {
            return FittedSizes::default();
        }
    }
    if fit == BoxFit::Fill {
        return FittedSizes::new(input, output);
    }
    if fit == BoxFit::None {
        let size = Size::new(
            input.width.min(output.width),
            input.height.min(output.height),
        );
        return FittedSizes::new(size, size);
    }

    // Do the ratio arithmetic at Flutter's precision: intermediate products
    // must not overflow even when all the input and result extents fit f32.
    let (iw, ih) = (f64::from(input.width), f64::from(input.height));
    let (ow, oh) = (f64::from(output.width), f64::from(output.height));
    let fitted = |width: f64, height: f64| Size::new(width as f32, height as f32);
    let wider = ow / oh > iw / ih;
    match (fit, wider) {
        (BoxFit::Contain | BoxFit::FitHeight, true) => {
            FittedSizes::new(input, fitted(iw * oh / ih, oh))
        }
        (BoxFit::Contain | BoxFit::FitWidth, false) => {
            FittedSizes::new(input, fitted(ow, ih * ow / iw))
        }
        (BoxFit::Cover | BoxFit::FitWidth, true) => {
            FittedSizes::new(fitted(iw, iw * oh / ow), output)
        }
        (BoxFit::Cover | BoxFit::FitHeight, false) => {
            FittedSizes::new(fitted(ih * ow / oh, ih), output)
        }
        (BoxFit::ScaleDown, _) => {
            let aspect = iw / ih;
            let (mut width, mut height) = (iw, ih);
            if height > oh {
                (width, height) = (oh * aspect, oh);
            }
            if width > ow {
                (width, height) = (ow, ow / aspect);
            }
            FittedSizes::new(input, fitted(width, height))
        }
        (BoxFit::Fill | BoxFit::None, _) => unreachable!("handled before ratio arithmetic"),
    }
}
