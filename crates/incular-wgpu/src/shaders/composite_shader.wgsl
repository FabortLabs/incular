
struct Out { @builtin(position) position: vec4<f32>, @location(0) uv: vec2<f32>, @location(1) @interpolate(flat) alpha: f32, @location(2) @interpolate(flat) color: vec4<f32>, @location(3) @interpolate(flat) mode: f32, @location(4) @interpolate(flat) clip_rect: vec4<f32>, @location(5) @interpolate(flat) clip_radii: vec4<f32>, @location(6) @interpolate(flat) clip_shape: u32 };
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
  out.clip_shape = u32(options.y);
  return out;
}
// Offscreen color is premultiplied. Multiplying both stored RGB and alpha by
// the group alpha exactly once preserves overlap semantics at the parent.
fn draw_color(input: Out) -> vec4<f32> {
  let sample = textureSample(group_texture, target_sampler, input.uv);
  let alpha = input.alpha * analytic_coverage(input.position.xy, input.clip_rect, input.clip_radii, input.clip_shape);
  if (input.mode > 1.5) {
    let in_source = all(input.uv >= vec2<f32>(0.)) && all(input.uv < vec2<f32>(1.));
    return select(input.color, sample, in_source) * alpha;
  }
  if (input.mode > 0.5) {
    let a = sample.a * input.color.a * alpha;
    return vec4<f32>(input.color.rgb * a, a);
  }
  return vec4<f32>(sample.rgb * alpha, sample.a * alpha);
}

@fragment fn fs_main(input: Out) -> @location(0) vec4<f32> { return draw_color(input); }
