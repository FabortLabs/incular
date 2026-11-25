
struct Out { @builtin(position) position: vec4<f32>, @location(0) color: vec4<f32> };
@vertex fn vs_main(@location(0) quad: vec2<f32>, @location(1) rect: vec4<f32>, @location(2) color: vec4<f32>) -> Out { var out: Out; out.position = vec4<f32>(rect.xy + quad * rect.zw, 0., 1.); out.color = color; return out; }
fn draw_color(input: Out) -> vec4<f32> { return input.color; }

@fragment fn fs_main(input: Out) -> @location(0) vec4<f32> { return draw_color(input); }
