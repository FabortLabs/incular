// Stencil mask for a clip path: the mesh's exact filled shape. The
// antialiasing extrusion is ignored, so fringe quads have no area and the
// interior sits on the outline itself.
@vertex fn vs_main(@location(0) local: vec2<f32>, @location(1) affine: vec4<f32>, @location(2) translation: vec4<f32>, @location(3) surface: vec4<f32>) -> @builtin(position) vec4<f32> {
  let physical = vec2<f32>(
    affine.x * local.x + affine.z * local.y + translation.x,
    affine.y * local.x + affine.w * local.y + translation.y,
  );
  return vec4<f32>(physical.x / surface.x * 2. - 1., 1. - physical.y / surface.y * 2., 0., 1.);
}
@fragment fn fs_main() -> @location(0) vec4<f32> { return vec4<f32>(0.); }
