// `stage.x`: 0 none, 1 linear-to-sRGB, 2 sRGB-to-linear, 3 blend with
// `stage.y` the `BlendMode::code` and `color` the premultiplied source
// (`ColorFilterStage::encoded`). The stage runs before the matrix.
struct Params { matrix: array<vec4<f32>, 5>, stage: vec4<u32>, color: vec4<f32> };
struct Out { @builtin(position) position: vec4<f32>, @location(0) uv: vec2<f32> };
@group(0) @binding(0) var source: texture_2d<f32>;
@group(0) @binding(1) var source_sampler: sampler;
@group(0) @binding(2) var<uniform> params: Params;
@vertex fn vs_main(@location(0) quad: vec2<f32>) -> Out {
  var out: Out;
  // Texture rows start at the top; clip-space Y increases upwards.
  out.position = vec4<f32>(quad.x * 2. - 1., 1. - quad.y * 2., 0., 1.);
  out.uv = quad;
  return out;
}
fn straight_rgb(value: vec4<f32>) -> vec3<f32> {
  let alpha = clamp(value.a, 0., 1.);
  let rgb = clamp(value.rgb / max(alpha, .000001), vec3<f32>(0.), vec3<f32>(1.));
  return select(vec3<f32>(0.), rgb, alpha > .000001);
}
fn linear_to_srgb(c: vec3<f32>) -> vec3<f32> {
  let v = clamp(c, vec3<f32>(0.), vec3<f32>(1.));
  return select(1.055 * pow(v, vec3<f32>(1. / 2.4)) - .055, v * 12.92, v <= vec3<f32>(.0031308));
}
fn srgb_to_linear(c: vec3<f32>) -> vec3<f32> {
  let v = clamp(c, vec3<f32>(0.), vec3<f32>(1.));
  return select(pow((v + .055) / 1.055, vec3<f32>(2.4)), v / 12.92, v <= vec3<f32>(.04045));
}
// The artistic formulas of `blend_shader.wgsl` (`blend_premultiplied`).
fn artistic(mode: u32, s: vec3<f32>, d: vec3<f32>) -> vec3<f32> {
  var out = s;
  if (mode == 11u) { out = s * d; }
  else if (mode == 12u) { out = s + d - s * d; }
  else if (mode == 13u) { out = select(2. * s * d, 1. - 2. * (1. - s) * (1. - d), d > vec3<f32>(.5)); }
  else if (mode == 14u) { out = min(s, d); }
  else if (mode == 15u) { out = max(s, d); }
  else if (mode == 16u) { out = select(min(d / max(1. - s, vec3<f32>(.000001)), vec3<f32>(1.)), vec3<f32>(1.), s >= vec3<f32>(1.)); }
  else if (mode == 17u) { out = select(max(1. - (1. - d) / max(s, vec3<f32>(.000001)), vec3<f32>(0.)), vec3<f32>(0.), s <= vec3<f32>(0.)); }
  else if (mode == 18u) { out = select(2. * s * d, 1. - 2. * (1. - s) * (1. - d), s > vec3<f32>(.5)); }
  else if (mode == 19u) {
    let low = d - (1. - 2. * s) * d * (1. - d);
    let g = select(sqrt(d), ((16. * d - 12.) * d + 4.) * d, d <= vec3<f32>(.25));
    out = select(low, d + (2. * s - 1.) * (g - d), s > vec3<f32>(.5));
  }
  else if (mode == 20u) { out = abs(d - s); }
  else if (mode == 21u) { out = s + d - 2. * s * d; }
  return clamp(out, vec3<f32>(0.), vec3<f32>(1.));
}
fn blend(mode: u32, src: vec4<f32>, dst: vec4<f32>) -> vec4<f32> {
  let as_ = clamp(src.a, 0., 1.);
  let ad = clamp(dst.a, 0., 1.);
  let ao = clamp(as_ + ad - as_ * ad, 0., 1.);
  var out = vec4<f32>(0.);
  if (mode == 0u) { out = vec4<f32>(src.rgb + dst.rgb * (1. - as_), ao); }
  else if (mode == 1u) { out = src; }
  else if (mode == 2u) { out = vec4<f32>(dst.rgb + src.rgb * (1. - ad), ao); }
  else if (mode == 3u) { out = vec4<f32>(src.rgb * ad, as_ * ad); }
  else if (mode == 4u) { out = vec4<f32>(dst.rgb * as_, ad * as_); }
  else if (mode == 5u) { out = vec4<f32>(src.rgb * (1. - ad), as_ * (1. - ad)); }
  else if (mode == 6u) { out = vec4<f32>(dst.rgb * (1. - as_), ad * (1. - as_)); }
  else if (mode == 7u) { out = vec4<f32>(src.rgb * ad + dst.rgb * (1. - as_), ad); }
  else if (mode == 8u) { out = vec4<f32>(dst.rgb * as_ + src.rgb * (1. - ad), as_); }
  else if (mode == 9u) { out = vec4<f32>(src.rgb * (1. - ad) + dst.rgb * (1. - as_), clamp(as_ + ad - 2. * as_ * ad, 0., 1.)); }
  else if (mode == 10u) { out = vec4<f32>(min(src.rgb + dst.rgb, vec3<f32>(1.)), min(as_ + ad, 1.)); }
  else if (mode == 22u) { out = src * dst; }
  else {
    let blended = artistic(mode, straight_rgb(src), straight_rgb(dst));
    out = vec4<f32>(clamp(src.rgb * (1. - ad) + dst.rgb * (1. - as_) + as_ * ad * blended, vec3<f32>(0.), vec3<f32>(1.)), ao);
  }
  let a = clamp(out.a, 0., 1.);
  return vec4<f32>(clamp(out.rgb, vec3<f32>(0.), vec3<f32>(a)), a);
}
@fragment fn fs_main(input: Out) -> @location(0) vec4<f32> {
  var sample = textureSampleLevel(source, source_sampler, input.uv, 0.);
  if (params.stage.x == 3u) {
    sample = blend(params.stage.y, params.color, clamp(sample, vec4<f32>(0.), vec4<f32>(1.)));
  }
  let a = clamp(sample.a, 0., 1.);
  var rgb = straight_rgb(sample);
  if (params.stage.x == 1u) { rgb = linear_to_srgb(rgb); }
  else if (params.stage.x == 2u) { rgb = srgb_to_linear(rgb); }
  let straight = select(vec4<f32>(0., 0., 0., a), vec4<f32>(rgb, a), a > .000001);
  let c0 = params.matrix[0];
  let c1 = params.matrix[1];
  let c2 = params.matrix[2];
  let c3 = params.matrix[3];
  let c4 = params.matrix[4];
  let filtered = vec4<f32>(
    dot(c0, straight) + c1.x,
    c1.y * straight.x + c1.z * straight.y + c1.w * straight.z + c2.x * straight.w + c2.y,
    c2.z * straight.x + c2.w * straight.y + c3.x * straight.z + c3.y * straight.w + c3.z,
    c3.w * straight.x + c4.x * straight.y + c4.y * straight.z + c4.z * straight.w + c4.w
  );
  let out_a = clamp(filtered.a, 0., 1.);
  let out_rgb = clamp(filtered.rgb, vec3<f32>(0.), vec3<f32>(1.)) * out_a;
  return vec4<f32>(out_rgb, out_a);
}
