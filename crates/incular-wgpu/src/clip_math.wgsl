// Shapes are expressed in their containing framebuffer's physical pixels.
// An empty rectangle in a composite instance means no additional mask.
fn analytic_coverage(point: vec2<f32>, rect: vec4<f32>, radii: vec4<f32>, shape: u32) -> f32 {
  if (rect.z <= 0. || rect.w <= 0.) {
    return 1.;
  }
  if (shape == 3u) {
    let q = point - rect.xy;
    let row_x = radii.xy;
    let row_y = radii.zw;
    let unit_point = vec2(dot(row_x, q), dot(row_y, q));
    let rho = length(unit_point);
    // The norm of the framebuffer gradient normalizes the implicit ellipse
    // to physical-pixel distance under rotation, nonuniform scale, or shear.
    let gradient = length(unit_point.x * row_x + unit_point.y * row_y) / max(rho, 1e-6);
    let distance = (rho - 1.0) / max(gradient, 1e-6);
    return clamp(0.5 - distance, 0.0, 1.0);
  }
  let q = point - rect.xy - rect.zw * 0.5;
  if (shape == 2u) {
    // Box coverage is the intersection area of the rect and the unit
    // pixel; both fractional edges contribute at a corner.
    let overlap = max(
      min(point + vec2(0.5), rect.xy + rect.zw) -
      max(point - vec2(0.5), rect.xy),
      vec2(0.0),
    );
    return overlap.x * overlap.y;
  }
  if (shape == 1u) {
    let radius = rect.zw * 0.5;
    let unit_point = q / radius;
    let rho = length(unit_point);
    // Normalize the ellipse's implicit gradient to measure distance in
    // physical pixels near the boundary, regardless of aspect ratio.
    let gradient = length(unit_point / radius) / max(rho, 1e-6);
    let distance = (rho - 1.0) / max(gradient, 1e-6);
    return clamp(0.5 - distance, 0.0, 1.0);
  }
  let top = select(radii.x, radii.y, q.x >= 0.);
  let bottom = select(radii.w, radii.z, q.x >= 0.);
  let radius = select(top, bottom, q.y >= 0.);
  let d = abs(q) - (rect.zw * 0.5 - vec2(radius));
  let distance = length(max(d, vec2(0.))) + min(max(d.x, d.y), 0.) - radius;
  return clamp(0.5 - distance, 0., 1.);
}
