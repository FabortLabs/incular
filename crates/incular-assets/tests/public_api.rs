use incular_assets::{FontHandle, FontId};

#[test]
fn collection_face_identity_is_retained() {
    let font = FontHandle::with_face_index(FontId(9), [1, 2, 3], 2);
    assert_eq!(font.id(), FontId(9));
    assert_eq!(font.face_index(), 2);
    assert_eq!(&**font.bytes(), &[1, 2, 3]);
}

#[test]
fn normalized_location_uses_face_order_and_preserves_font_identity() {
    use incular_assets::FontLocation;

    let left = FontLocation::from_normalized_coordinates(&[0, -8_192, 0]);
    let equivalent = FontLocation::from_normalized_coordinates(&[0, -8_192]);
    assert_eq!(left, equivalent);
    assert_eq!(left.coordinates(), &[0, -8_192]);
    assert_ne!(
        left,
        FontLocation::from_normalized_coordinates(&[-8_192, 0])
    );
    assert!(FontLocation::from_normalized_coordinates(&[0, 0]).is_default());

    let face = FontHandle::with_face_index(FontId(1234), [1, 2, 3], 2);
    let varied = face.clone().with_location(left);
    assert_eq!(face.id(), varied.id());
    assert_eq!(face.face_index(), varied.face_index());
    assert_eq!(face.bytes().as_ref(), varied.bytes().as_ref());
    assert_ne!(face, varied);
    assert!(face.location().is_default());
}
