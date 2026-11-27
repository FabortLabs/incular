//! Final shader source assembly, shared by the build and embedded-IR test.

pub fn with_shared_functions(name: &str, base: String) -> String {
    let shared = match name {
        "blend_shader" | "color_matrix_shader" => include_str!("src/artistic_blend.wgsl"),
        "composite_shader" => include_str!("src/clip_math.wgsl"),
        _ => return base,
    };
    format!("{base}\n{shared}")
}

pub fn with_soft_clip(name: &str, base: String, native_d3d11: bool) -> String {
    if native_d3d11
        || !matches!(
            name,
            "rect_shader"
                | "square_rect_shader"
                | "rrect_shader"
                | "path_shader"
                | "image_shader"
                | "text_shader"
                | "composite_shader"
        )
    {
        return base;
    }

    // Composite already shares clip math and outputs premultiplied color.
    // The other source shaders output straight color and only weight alpha.
    let (math, coverage) = if name == "composite_shader" {
        ("", "color * weight")
    } else {
        (
            include_str!("src/clip_math.wgsl"),
            "vec4<f32>(color.rgb, color.a * weight)",
        )
    };
    format!(
        "{base}\n{math}\n{}\n@fragment fn fs_clipped(input: Out) -> @location(0) vec4<f32> {{ let color = draw_color(input); let weight = clip_weight(input.position.xy); return {coverage}; }}",
        include_str!("src/soft_clip.wgsl")
    )
}
