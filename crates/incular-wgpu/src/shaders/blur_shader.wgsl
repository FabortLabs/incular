struct Params {
  source_origin: vec2<f32>,
  source_size: vec2<f32>,
  output_size: vec2<f32>,
  direction: vec2<f32>,
  // Bilinear tap pairs after the centre tap.
  radius: u32,
  mode: u32,
  _padding: vec2<u32>,
  // [0].x: centre weight. [p], p >= 1: x = first tap k of the pair, y = the
  // pair's linear-filtered offset (k*w_k + (k+1)*w_k1) / (w_k + w_k1),
  // z = w_k, w = w_k1 (zero for a lone last tap).
  weights: array<vec4<f32>, 52>,
};
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
fn inside(px: vec2<f32>) -> bool {
  return px.x >= 0. && px.y >= 0. && px.x < params.source_size.x && px.y < params.source_size.y;
}
// The source at `px` (texel units, texel k at [k, k + 1)), bilinear between
// texel centres; transparent outside the source.
fn fetch(px: vec2<f32>) -> vec4<f32> {
  return textureSampleLevel(source, source_sampler, (px + vec2<f32>(0.5)) / params.source_size, 0.);
}
// Taps k and k + 1 on one side (`side` = +1 or -1) of `base`. One linear
// fetch between them when both lie in the source; otherwise only the one
// inside, at its own centre, so the transparent edge never blends in.
fn pair(base: vec2<f32>, tap: vec4<f32>, side: f32) -> vec4<f32> {
  let first = base + params.direction * (side * tap.x);
  let second = base + params.direction * (side * (tap.x + 1.));
  let first_in = inside(first);
  let second_in = tap.w == 0. || inside(second);
  if (first_in && second_in) {
    return fetch(base + params.direction * (side * tap.y)) * (tap.z + tap.w);
  }
  if (first_in) {
    return fetch(first) * tap.z;
  }
  if (inside(second)) {
    return fetch(second) * tap.w;
  }
  return vec4<f32>(0.);
}
@fragment fn fs_main(input: Out) -> @location(0) vec4<f32> {
  // Interpolated UV addresses output pixel centres (j + 0.5). Floor to the
  // pixel index: subtracting 0.5 instead can land a hair below zero and
  // drop the first source row and column as out of range.
  let base = floor(input.uv * params.output_size) - params.source_origin;
  // The farthest tap on either side. The source is a rectangle, so when both
  // ends lie inside it every tap between them does too: interior pixels,
  // nearly all of them, take the same fetches without the per-tap edge tests.
  let reach = params.direction * (params.weights[params.radius].x + 1.);
  if (params.radius > 0u && inside(base - reach) && inside(base + reach)) {
    var sum = fetch(base) * params.weights[0].x;
    for (var p: u32 = 1u; p <= params.radius; p = p + 1u) {
      let tap = params.weights[p];
      let offset = params.direction * tap.y;
      let weight = tap.z + tap.w;
      sum += fetch(base + offset) * weight + fetch(base - offset) * weight;
    }
    return sum;
  }
  var result = vec4<f32>(0.);
  if (inside(base)) {
    result = fetch(base) * params.weights[0].x;
  }
  for (var p: u32 = 1u; p <= params.radius; p = p + 1u) {
    let tap = params.weights[p];
    result += pair(base, tap, 1.) + pair(base, tap, -1.);
  }
  return result;
}
