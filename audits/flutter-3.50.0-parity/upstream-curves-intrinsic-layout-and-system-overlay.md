# Curves, intrinsic sizing, Align, and system overlays

Pinned Flutter 3.50.0 `ab2912035e`; unchanged Incular production `c647cb85`.

- Five original animation curve registrations match fully
  (:45, :102, :172, :231, :471). Tests cover sampled continuity of
  34 curves, three bounce variants, the exact compound midpoint,
  15 exact start/end pairs and 8 Catmull–Rom samples at 1e-6.
  No claim about protected transformInternal handling.
  See `flutter350_upstream_curve_samples`.
- IntrinsicWidth with optional/zero steps measures 100×50 in
  both. Flutter debug rejects negative width/height steps
  while the Rust public setters accept them (:9). The zero
  area control (:35) compares equivalent tight constraints,
  not OS resize. Align checks (:13, :25) reproduce Flutter's
  expansion and shrink-wrapping dimensions, through public widget
  rather than original direct RenderObject geometry queries.
  All four are partial. See `flutter_intrinsic_align_geometry_parity`.
- System-overlay tests :68/:89/:194 compare host-selected
  overlay style only. Annotation height 11.5 misses the
  12.5 status sample and height 13.5 includes it. The upper
  status and lower navigation annotations merge their colors.
  Original variants passed; three Rust checks match the selected
  payload. Partial: no Dart platform channel, native status
  or navigation rendering, or Fuchsia Rust mobile host claim.
  See `flutter350_system_overlay_annotations_parity`.
