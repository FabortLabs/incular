
struct Params {
  source_origin: vec2<f32>,
  source_size: vec2<f32>,
  output_size: vec2<f32>,
  direction: vec2<f32>,
  radius: u32,
  mode: u32,
  _padding: vec2<u32>,
  weights: array<vec4<f32>, 16>,
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
@fragment fn fs_main(input: Out) -> @location(0) vec4<f32> {
  // UV already addresses output pixel centers. Keep source coordinates in
  // that convention so upsampling retains coverage at the first texel.
  let px = input.uv * params.output_size * params.direction - params.source_origin;
  if (px.x < 0. || px.y < 0. || px.x >= params.source_size.x || px.y >= params.source_size.y) {
    return vec4<f32>(0.);
  }
  return textureSampleLevel(source, source_sampler, px / params.source_size, 0.);
}
