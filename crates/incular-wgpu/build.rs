mod shader_source;

use std::fmt::Write as _;
use std::{env, fs, path::PathBuf};

fn main() {
    println!("cargo:rustc-check-cfg=cfg(incular_d3d11)");
    let native = env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows")
        && env::var_os("CARGO_FEATURE_D3D11").is_some()
        && ["DX12", "VULKAN", "GL", "GPU_PROFILING"]
            .iter()
            .all(|name| env::var_os(format!("CARGO_FEATURE_{name}")).is_none());
    if native {
        println!("cargo:rustc-cfg=incular_d3d11");
    }
    let source_dir = PathBuf::from("src/shaders");
    println!("cargo:rerun-if-changed={}", source_dir.display());
    println!("cargo:rerun-if-changed=src/soft_clip.wgsl");
    println!("cargo:rerun-if-changed=src/clip_math.wgsl");
    println!("cargo:rerun-if-changed=src/artistic_blend.wgsl");
    let output = PathBuf::from(env::var_os("OUT_DIR").expect("Cargo provides OUT_DIR"));
    let mut paths: Vec<_> = fs::read_dir(&source_dir)
        .expect("built-in shader directory")
        .map(|entry| entry.expect("shader directory entry").path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "wgsl")
        })
        .collect();
    paths.sort();
    let mut bindings = String::new();
    #[cfg(feature = "d3d11")]
    let mut native_bindings = String::new();
    for path in paths {
        let base = fs::read_to_string(&path).expect("read built-in shader");
        let name = path
            .file_stem()
            .expect("shader stem")
            .to_str()
            .expect("UTF-8 shader name");
        let base = shader_source::with_shared_functions(name, base);
        #[cfg(feature = "d3d11")]
        if native && name != "bootstrap_shader" {
            fs::write(output.join(format!("{name}.hlsl")), direct3d_shader(&base))
                .expect("write D3D11 shader");
            writeln!(native_bindings,r#"pub(crate) const {}: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/{name}.hlsl"));"#,name.to_ascii_uppercase()).expect("write to String");
        }
        let source = shader_source::with_soft_clip(name, base, native);
        let module = naga::front::wgsl::parse_str(&source).unwrap_or_else(|error| {
            panic!("{}: {}", path.display(), error.emit_to_string(&source))
        });
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::all(),
        )
        .validate(&module)
        .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
        let bytes = postcard::to_allocvec(&module).expect("serialize built-in shader");
        fs::write(output.join(format!("{name}.naga")), bytes).expect("write shader IR");
        writeln!(
            bindings,
            "pub(crate) const {}: &[u8] = include_bytes!(concat!(env!(\"OUT_DIR\"), \"/{name}.naga\"));",
            name.to_ascii_uppercase()
        )
        .expect("write to String cannot fail");
    }
    #[cfg(feature = "d3d11")]
    if native {
        fs::write(output.join("d3d11_shaders.rs"), native_bindings)
            .expect("write D3D11 shader bindings");
    }
    fs::write(output.join("built_in_shaders.rs"), bindings).expect("write shader bindings");
}

#[cfg(feature = "d3d11")]
fn direct3d_shader(source: &str) -> String {
    use naga_d3d11::{
        back::hlsl,
        front::wgsl,
        valid::{Capabilities, ValidationFlags, Validator},
    };
    let module = wgsl::parse_str(source).expect("shader is compatible with Shader Model 5");
    let info = Validator::new(ValidationFlags::all(), Capabilities::all())
        .validate(&module)
        .expect("valid Shader Model 5 module");
    let options = hlsl::Options {
        shader_model: hlsl::ShaderModel::V5_0,
        ..Default::default()
    };
    let mut output = String::new();
    let reflection = hlsl::Writer::new(&mut output, &options)
        .write(&module, &info, None)
        .expect("translate built-in shader to HLSL");
    for (entry, name) in module.entry_points.iter().zip(reflection.entry_point_names) {
        assert_eq!(
            name.as_deref().ok(),
            Some(entry.name.as_str()),
            "stable shader entry point"
        );
    }
    output
}
