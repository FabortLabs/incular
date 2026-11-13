struct Params {
  source_origin: vec2<f32>,
  source_size: vec2<f32>,
  output_size: vec2<f32>,
  direction: vec2<f32>,
  radius: u32,
  mode: u32,
  _padding: vec2<u32>,
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
// One axis of a 2-texel box tap starting at texel `start`: the sample
// coordinate and how many of the two texels lie inside `size`. Both inside
// samples the boundary between them (a 50/50 bilinear mix); one inside
// samples that texel's centre, so the transparent outside never blends in.
fn box_axis(start: f32, size: f32) -> vec2<f32> {
  let first = start >= 0. && start < size;
  let second = start + 1. >= 0. && start + 1. < size;
  if (first && second) {
    return vec2<f32>(start + 1., 2.);
  }
  if (first) {
    return vec2<f32>(start + 0.5, 1.);
  }
  if (second) {
    return vec2<f32>(start + 1.5, 1.);
  }
  return vec2<f32>(0., 0.);
}
@fragment fn fs_main(input: Out) -> @location(0) vec4<f32> {
  if (params.mode == 3u) {
    // Box downsample by `direction` (the factor): the average of the
    // factor x factor source block under this output pixel, in 2x2
    // bilinear taps, transparent outside the source.
    let factor = params.direction;
    let start = floor(input.uv * params.output_size) * factor - params.source_origin;
    let taps = max(vec2<i32>(factor * 0.5), vec2<i32>(1));
    // One reciprocal for every tap of the block.
    let texel = 1. / params.source_size;
    var sum = vec4<f32>(0.);
    for (var y: i32 = 0; y < taps.y; y = y + 1) {
      let row = box_axis(start.y + 2. * f32(y), params.source_size.y);
      if (row.y == 0.) {
        continue;
      }
      for (var x: i32 = 0; x < taps.x; x = x + 1) {
        let column = box_axis(start.x + 2. * f32(x), params.source_size.x);
        if (column.y == 0.) {
          continue;
        }
        let uv = vec2<f32>(column.x, row.x) * texel;
        sum += textureSampleLevel(source, source_sampler, uv, 0.) * (column.y * row.y);
      }
    }
    return sum / (f32(taps.x) * f32(taps.y) * 4.);
  }
  // UV already addresses output pixel centers. Keep source coordinates in
  // that convention so upsampling retains coverage at the first texel.
  var px = input.uv * params.output_size * params.direction - params.source_origin;
  if (params.mode == 2u) {
    // An affine resample (a backdrop ImageFilter.matrix): the output pixel
    // position maps through [a c e; b d f], packed in the first weights.
    let position = input.uv * params.output_size;
    let linear = params.weights[0];
    let offset = params.weights[1].xy;
    px = vec2<f32>(
      linear.x * position.x + linear.z * position.y,
      linear.y * position.x + linear.w * position.y,
    ) + offset;
  }
  if (px.x < 0. || px.y < 0. || px.x >= params.source_size.x || px.y >= params.source_size.y) {
    return vec4<f32>(0.);
  }
  return textureSampleLevel(source, source_sampler, px / params.source_size, 0.);
}
