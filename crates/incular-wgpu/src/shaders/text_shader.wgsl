
// `color` and `tone` are constant across a glyph quad and passed flat. `tone`
// carries the half of the preblend that depends on the text color alone, so
// it is evaluated per vertex instead of per covered pixel.
struct Out { @builtin(position) position: vec4<f32>, @location(0) uv: vec2<f32>, @location(1) @interpolate(flat, either) color: vec4<f32>, @location(2) @interpolate(flat, either) tone: vec4<f32> };
@group(0) @binding(0) var atlas: texture_2d<f32>;
@group(0) @binding(1) var atlas_sampler: sampler;
fn srgb_to_linear(v: f32) -> f32 { if (v <= .04045) { return v / 12.92; } return pow((v + .055) / 1.055, 2.4); }
fn linear_to_srgb(v: f32) -> f32 { if (v <= .0031308) { return v * 12.92; } return 1.055 * pow(v, 1. / 2.4) - .055; }
// Skia's A8 glyph preblend (SkTMaskGamma_build_correcting_lut, contrast 0.5,
// sRGB gamma): the text color's luminance, quantized to three bits, picks a
// table that boosts light text and adds contrast to dark text.
//
// The color-only terms: x = src, y = dst, z = linear src, w = linear dst.
fn preblend_tone(rgb: vec3<f32>) -> vec4<f32> {
    let c = round(clamp(rgb, vec3(0.), vec3(1.)) * 255.);
    let lum = floor((c.r * 54. + c.g * 183. + c.b * 19.) / 256.);
    let q = floor(lum / 32.);
    let src = (q * 32. + q * 4. + floor(q / 2.)) / 255.;
    let dst = 1. - src;
    return vec4(src, dst, srgb_to_linear(src), srgb_to_linear(dst));
}
fn preblend(raw: f32, tone: vec4<f32>) -> f32 {
    let src = tone.x;
    let dst = tone.y;
    let a = round(clamp(raw, 0., 1.) * 255.) / 255.;
    let srca = a + (1. - a) * .5 * tone.w * a;
    if (abs(src - dst) < 1. / 256.) { return round(255. * srca) / 255.; }
    let lin_out = tone.z * srca + (1. - srca) * tone.w;
    let out = linear_to_srgb(lin_out);
    return clamp(round(255. * (out - dst) / (src - dst)) / 255., 0., 1.);
}
@vertex fn vs_main(@location(0) quad: vec2<f32>, @location(1) rect: vec4<f32>, @location(2) affine: vec4<f32>, @location(3) translation: vec4<f32>, @location(4) surface: vec4<f32>, @location(5) uv: vec4<f32>, @location(6) color: vec4<f32>) -> Out { var out: Out; let local=rect.xy+quad*rect.zw; let p=vec2<f32>(affine.x*local.x+affine.z*local.y+translation.x,affine.y*local.x+affine.w*local.y+translation.y); out.position=vec4<f32>(p.x/surface.x*2.-1.,1.-p.y/surface.y*2.,0.,1.); out.uv=uv.xy+quad*(uv.zw-uv.xy); out.color=color; out.tone=preblend_tone(color.rgb); return out; }
@fragment fn fs_main(input: Out) -> @location(0) vec4<f32> { let coverage = preblend(textureSample(atlas, atlas_sampler, input.uv).r, input.tone); return vec4<f32>(input.color.rgb, input.color.a * coverage); }

