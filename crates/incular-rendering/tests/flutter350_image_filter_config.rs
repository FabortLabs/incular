use std::hash::{DefaultHasher, Hash, Hasher};

use incular_core::{Offset, Rect, Size};
use incular_rendering::{
    ImageFilter, ImageFilterConfig as Config, ImageFilterContext as Context, ImageFilterTarget,
    TileMode, backdrop_image_filter_executable,
};

fn digest(value: &impl Hash) -> u64 {
    let mut state = DefaultHasher::new();
    value.hash(&mut state);
    state.finish()
}

#[test]
fn context_and_direct_filters_keep_their_original_values_diagnostics_and_hash() {
    let context = Context {
        bounds: Rect::default(),
    };
    assert_eq!(context.bounds, Rect::default());
    let filter = ImageFilter::blur(5.1, 5.1);
    let direct = Config::new(filter.clone());
    let same = Config::new(ImageFilter::blur(5.1, 5.1));
    let different = Config::new(ImageFilter::blur(10.1, 10.1));
    assert_eq!(direct, same);
    assert_eq!(digest(&direct), digest(&same));
    assert_ne!(direct, different);
    assert_ne!(digest(&direct), digest(&different));
    assert_eq!(different, Config::new(ImageFilter::blur(10.1, 10.1)));
    assert_eq!(
        digest(&different),
        digest(&Config::new(ImageFilter::blur(10.1, 10.1)))
    );
    assert_eq!(
        direct.debug_short_description(),
        "blur(5.1, 5.1, unspecified)"
    );
    assert_eq!(
        direct.to_string(),
        "ImageFilterConfig(blur(5.1, 5.1, unspecified))"
    );
    assert_eq!(
        direct.filter().unwrap().to_string(),
        "ImageFilter.blur(5.1, 5.1, unspecified)"
    );
    assert_eq!(direct.resolve(context), filter);
}

#[test]
fn configured_blur_and_composition_preserve_sampling_and_direction_in_values() {
    let mild = Config::blur(5.1, 5.1);
    let strong = Config::blur(10.1, 10.1);
    let bounded = Config::blur_with_sampling(5.1, 5.1, TileMode::Clamp, true);
    assert_eq!(mild, Config::blur(5.1, 5.1));
    assert_eq!(digest(&mild), digest(&Config::blur(5.1, 5.1)));
    assert_ne!(mild, strong);
    assert_ne!(mild, bounded);
    assert_ne!(digest(&mild), digest(&strong));
    assert_ne!(digest(&mild), digest(&bounded));
    assert_eq!(
        digest(&Config::blur(-0., 0.)),
        digest(&Config::blur(0., -0.))
    );
    assert!(bounded.filter().is_none());

    let decal = Config::blur_with_sampling(2.5, 3.5, TileMode::Decal, false);
    let glass = Config::blur_with_sampling(2.5, 3.5, TileMode::Clamp, true);
    assert_eq!(
        decal.debug_short_description(),
        "blur(2.5, 3.5, decal, unbounded)"
    );
    assert_eq!(
        decal.to_string(),
        "ImageFilterConfig.blur(2.5, 3.5, decal, unbounded)"
    );
    assert_eq!(
        glass.debug_short_description(),
        "blur(2.5, 3.5, clamp, bounded)"
    );
    assert_eq!(
        glass.to_string(),
        "ImageFilterConfig.blur(2.5, 3.5, clamp, bounded)"
    );

    let composed = Config::compose(mild.clone(), strong.clone());
    let reverse = Config::compose(strong.clone(), mild.clone());
    assert_eq!(composed, Config::compose(mild.clone(), strong.clone()));
    assert_eq!(
        digest(&composed),
        digest(&Config::compose(mild.clone(), strong.clone()))
    );
    assert_ne!(composed, reverse);
    assert_ne!(digest(&composed), digest(&reverse));
    let short = format!(
        "{} -> {}",
        strong.debug_short_description(),
        mild.debug_short_description()
    );
    assert_eq!(composed.debug_short_description(), short);
    assert_eq!(
        composed.to_string(),
        format!("ImageFilterConfig.compose(source -> {short} -> result)")
    );
}

#[test]
fn layout_bounds_resolve_as_sampling_metadata_at_each_stage() {
    let context = Context {
        bounds: Rect::from_origin_size(Offset::new(0.1, 0.1), Size::new(100.1, 100.1)),
    };
    let unbounded = Config::blur(10.1, 10.1);
    let bounded = Config::blur_with_sampling(10.1, 10.1, TileMode::Clamp, true);
    let blur = unbounded.resolve(context);
    assert_eq!(blur.debug_short_description(), "blur(10.1, 10.1, clamp)");
    let sampled = bounded.resolve(context);
    let description = "blur(10.1, 10.1, clamp, bounds: Rect.fromLTRB(0.1, 0.1, 100.2, 100.2))";
    assert_eq!(sampled.debug_short_description(), description);
    let ImageFilter::SampledBlur(parameters) = &sampled else {
        panic!("expected a sampling-aware blur")
    };
    assert_eq!(parameters.bounds, Some(context.bounds));
    let resolved = Config::compose(unbounded, bounded).resolve(context);
    assert_eq!(
        resolved.debug_short_description(),
        format!("{description} -> blur(10.1, 10.1, clamp)")
    );
    assert_eq!(resolved.stages(), [sampled.clone(), blur]);
    let unsupported = backdrop_image_filter_executable(&resolved).unwrap_err();
    assert_eq!(unsupported.target, ImageFilterTarget::Backdrop);
    assert_eq!(unsupported.filter, sampled);
}
