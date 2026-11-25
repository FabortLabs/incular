// Each rounded clip is expressed in its containing framebuffer's physical
// pixels. Index zero is the empty stack; other entries link to their parent.
struct SoftClip {
  rect: vec4<f32>,
  radii: vec4<f32>,
  link: vec4<u32>,
}
@group(1) @binding(0) var<storage, read> soft_clips: array<SoftClip>;
@group(1) @binding(1) var<uniform> soft_clip_index: vec4<u32>;

fn oval_coverage(q: vec2<f32>, size: vec2<f32>) -> f32 {
  let radius = size * 0.5;
  let unit_point = q / radius;
  let rho = length(unit_point);
  // Divide the implicit ellipse by its gradient to measure a physical
  // pixel near the edge, irrespective of which axis is the longer one.
  let gradient = length(unit_point / radius) / max(rho, 1e-6);
  let distance = (rho - 1.0) / max(gradient, 1e-6);
  return clamp(0.5 - distance, 0.0, 1.0);
}

fn clip_weight(point: vec2<f32>) -> f32 {
  var index = soft_clip_index.x;
  var weight = 1.0;
  while (index != 0u) {
    let mask = soft_clips[index];
    let q = point - mask.rect.xy - mask.rect.zw * 0.5;
    if (mask.link.y == 1u) {
      weight *= oval_coverage(q, mask.rect.zw);
    } else {
      let top = select(mask.radii.x, mask.radii.y, q.x >= 0.0);
      let bottom = select(mask.radii.w, mask.radii.z, q.x >= 0.0);
      let radius = select(top, bottom, q.y >= 0.0);
      let d = abs(q) - (mask.rect.zw * 0.5 - vec2(radius));
      let signed_distance = length(max(d, vec2(0.0))) + min(max(d.x, d.y), 0.0) - radius;
      weight *= clamp(0.5 - signed_distance, 0.0, 1.0);
    }
    if (weight == 0.0) {
      break;
    }
    index = mask.link.x;
  }
  return weight;
}
