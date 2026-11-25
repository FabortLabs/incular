
struct Out { @builtin(position) position: vec4<f32>, @location(0) uv: vec2<f32>, @location(1) @interpolate(flat) alpha: f32, @location(2) @interpolate(flat) color: vec4<f32>, @location(3) @interpolate(flat) mode: f32, @location(4) @interpolate(flat) clip_rect: vec4<f32>, @location(5) @interpolate(flat) clip_radii: vec4<f32> };
@group(0) @binding(0) var group_texture: texture_2d<f32>;
@group(0) @binding(1) var target_sampler: sampler;
@vertex fn vs_main(@location(0) quad: vec2<f32>, @location(1) rect: vec4<f32>, @location(2) uv: vec4<f32>, @location(3) alpha: vec4<f32>, @location(4) color: vec4<f32>, @location(5) options: vec4<f32>, @location(6) clip_rect: vec4<f32>, @location(7) clip_radii: vec4<f32>) -> Out {
  var out: Out;
  out.position = vec4<f32>(rect.xy + quad * rect.zw, 0., 1.);
  out.uv = uv.xy + quad * (uv.zw - uv.xy);
  out.alpha = alpha.x;
  out.color = color;
  out.mode = options.x;
  out.clip_rect = clip_rect;
  out.clip_radii = clip_radii;
  return out;
}
fn rounded_coverage(p: vec2<f32>, rect: vec4<f32>, radii: vec4<f32>) -> f32 {
  if (rect.z <= 0. || rect.w <= 0.) {
    return 1.;
  }
  let q = p - rect.xy - rect.zw * 0.5;
  let top = select(radii.x, radii.y, q.x >= 0.);
  let bottom = select(radii.w, radii.z, q.x >= 0.);
  let radius = select(top, bottom, q.y >= 0.);
  let d = abs(q) - (rect.zw * 0.5 - vec2(radius));
  let distance = length(max(d, vec2(0.))) + min(max(d.x, d.y), 0.) - radius;
  return clamp(0.5 - distance, 0., 1.);
}
// Offscreen color is premultiplied. Multiplying both stored RGB and alpha by
// the group alpha exactly once preserves overlap semantics at the parent.
@fragment fn fs_main(input: Out) -> @location(0) vec4<f32> {
  let sample = textureSample(group_texture, target_sampler, input.uv);
  let alpha = input.alpha * rounded_coverage(input.position.xy, input.clip_rect, input.clip_radii);
  if (input.mode > 0.5) {
    let a = sample.a * input.color.a * alpha;
    return vec4<f32>(input.color.rgb * a, a);
  }
  return vec4<f32>(sample.rgb * alpha, sample.a * alpha);
}
