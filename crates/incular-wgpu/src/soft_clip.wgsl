// Each analytic clip is expressed in its containing framebuffer's physical
// pixels. Index zero is the empty stack; other entries link to their parent.
struct SoftClip {
  rect: vec4<f32>,
  radii: vec4<f32>,
  link: vec4<u32>,
}
@group(1) @binding(0) var<storage, read> soft_clips: array<SoftClip>;
@group(1) @binding(1) var<uniform> soft_clip_index: vec4<u32>;

fn clip_weight(point: vec2<f32>) -> f32 {
  var index = soft_clip_index.x;
  var weight = 1.0;
  while (index != 0u) {
    let mask = soft_clips[index];
    weight *= analytic_coverage(point, mask.rect, mask.radii, mask.link.y);
    if (weight == 0.0) {
      break;
    }
    index = mask.link.x;
  }
  return weight;
}
