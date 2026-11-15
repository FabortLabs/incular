//! `AssetImage` density-variant selection, following Flutter's
//! `_findBestVariant` (`painting/image_resolution.dart`). The examples in the
//! doc comments of that file (1.25 gives 2.0x, 2.25 gives 2.0x, 2.75 gives
//! 3.0x, 3.25 gives 3.0x) are asserted directly.

use std::fs;
use std::path::PathBuf;

use incular_image::AssetImage;

struct Fixture {
    root: PathBuf,
}

impl Fixture {
    fn new(name: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "incular-asset-variants-{}-{name}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("create fixture root");
        Self { root }
    }

    fn touch(&self, relative: &str) {
        let path = self.root.join(relative);
        fs::create_dir_all(path.parent().expect("parent")).expect("create directory");
        fs::write(path, b"variant").expect("write variant");
    }

    fn asset(&self) -> AssetImage {
        AssetImage::new(self.root.join("heart.png"))
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn main_asset_without_variants_is_used_at_every_ratio() {
    let fixture = Fixture::new("none");
    fixture.touch("heart.png");
    for ratio in [1.0, 2.0, 3.0] {
        assert_eq!(
            fixture.asset().variant_path(ratio),
            fixture.root.join("heart.png")
        );
    }
}

#[test]
fn flutter_documented_examples_select_the_expected_variant() {
    let fixture = Fixture::new("documented");
    fixture.touch("heart.png");
    fixture.touch("2.0x/heart.png");
    fixture.touch("3.0x/heart.png");
    let asset = fixture.asset();
    let expected = [
        (1.0, "heart.png"),
        // Below the low-DPR limit (2.0) the higher variant wins.
        (1.25, "2.0x/heart.png"),
        (2.0, "2.0x/heart.png"),
        // Past the midpoint (2.5) between 2.0 and 3.0 the higher one wins.
        (2.25, "2.0x/heart.png"),
        (2.75, "3.0x/heart.png"),
        (3.0, "3.0x/heart.png"),
        // Above the highest key, the highest key is chosen.
        (3.25, "3.0x/heart.png"),
    ];
    for (ratio, relative) in expected {
        assert_eq!(
            asset.variant_path(ratio),
            fixture.root.join(relative),
            "device pixel ratio {ratio}"
        );
    }
}

#[test]
fn invalid_ratios_fall_back_to_natural_resolution() {
    let fixture = Fixture::new("invalid");
    fixture.touch("heart.png");
    fixture.touch("2.0x/heart.png");
    for ratio in [0.0, -2.0, f64::NAN, f64::INFINITY] {
        assert_eq!(
            fixture.asset().variant_path(ratio),
            fixture.root.join("heart.png"),
            "ratio {ratio}"
        );
    }
}

#[test]
fn variants_must_contain_the_same_file_name() {
    let fixture = Fixture::new("names");
    fixture.touch("heart.png");
    fixture.touch("2.0x/star.png");
    assert_eq!(
        fixture.asset().variant_path(2.0),
        fixture.root.join("heart.png"),
        "a variant of another file is not a variant of this one"
    );
}

#[test]
fn highest_variant_serves_ratios_above_it() {
    let fixture = Fixture::new("highest");
    fixture.touch("heart.png");
    fixture.touch("2.0x/heart.png");
    // The main asset is the exact match at 1.0; above the highest key (2.0)
    // the highest variant is chosen.
    assert_eq!(
        fixture.asset().variant_path(1.0),
        fixture.root.join("heart.png")
    );
    assert_eq!(
        fixture.asset().variant_path(4.0),
        fixture.root.join("2.0x/heart.png")
    );
}
