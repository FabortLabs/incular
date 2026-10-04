# Incular

[![Build](https://github.com/FabortLabs/incular/actions/workflows/build.yml/badge.svg)](https://github.com/FabortLabs/incular/actions/workflows/build.yml)

**Build native desktop apps in Rust with a declarative, retained UI framework.**

Incular combines composable widgets, reactive state, and a retained rendering pipeline. Describe your interface in Rust, connect it to application state, and let Incular update and render the parts that change. Its shared Winit desktop host and WGPU renderer run on Windows, Linux, and macOS.

## Architecture and support

| Contract | Status |
| --- | --- |
| Ownership | Feature-controlled public facade and application preludes. |
| API class | application; re-exports retain the defining API class under the [architecture contract](https://github.com/FabortLabs/incular/blob/master/system-design/ARCHITECTURE.md). |
| Support | Re-exports inherit original API classes; platform features are target-specific. |

## What you can build

- **Reactive interfaces:** `Signal`, `Memo`, `Effect`, and `Action` for state, derived values, side effects, and asynchronous operations.
- **Rich desktop UI:** flexible layout, text and text editing, images, scrolling lists, forms, and custom canvas painting.
- **Interactive experiences:** pointer gestures, keyboard input, focus management, transitions, and animation.
- **Accessible applications:** platform-neutral semantics projected into native accessibility systems.
- **Layered design systems:** neutral widgets with optional themed Controls and Material components.
- **Full application flows:** routing, overlays, restoration, windows, and native desktop services.
- **Runtime diagnostics:** opt-in instrumentation and a standalone DevTools application.

## A first window

```rust,no_run
use incular::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app = Application::new(|_cx| {
        Container::builder()
            .padding(EdgeInsets::all(24.0))
            .alignment(Alignment::CENTER)
            .child(Text::new("Hello, Incular!"))
            .build()
            .into()
    })?;

    incular::run(app)?;
    Ok(())
}
```

## Try Incular

You’ll need Rust **1.89 or newer**, a desktop environment, and a compatible GPU driver. Platform-specific setup is covered in the [installation guide](https://github.com/FabortLabs/incular/blob/master/docs/content/docs/introduction/installation.mdx).

```text
git clone https://github.com/FabortLabs/incular.git
cd incular
cargo run -p incular --example hello
```

The checkout includes a [gallery of examples](https://github.com/FabortLabs/incular/blob/master/examples/README.md), from controls and animation to navigation and custom rendering.

## Documentation

- [What is Incular?](https://github.com/FabortLabs/incular/blob/master/docs/content/docs/introduction/what-is-incular.mdx) — the framework’s mental model
- [Quick start](https://github.com/FabortLabs/incular/blob/master/docs/content/docs/introduction/quick-start.mdx) — build an app and add reactive state
- [API overview](https://github.com/FabortLabs/incular/blob/master/docs/content/docs/api/overview.mdx) — modules, preludes, and feature flags
- [Example gallery](https://github.com/FabortLabs/incular/blob/master/examples/README.md)
- [Architecture](https://github.com/FabortLabs/incular/blob/master/system-design/ARCHITECTURE.md) — crate responsibilities and design boundaries

## Feature flags

| Feature | Purpose | Default |
| --- | --- | --- |
| `desktop` | Native desktop runner with every WGPU backend | Yes |
| `desktop-host` | Native desktop runner; pair it with at least one backend below | Via `desktop` |
| `dx12`, `metal`, `vulkan`, `gl` | WGPU backends; each enables `desktop-host` and is ignored where unavailable | Via `desktop` |
| `controls` | Themed Incular controls | Yes |
| `material` | Material presentation; enables `controls` | Yes |
| `devtools` | Diagnostics transport and runtime instrumentation | No |

The main `incular::prelude` contains the neutral framework API. Import themed controls from `incular::controls_prelude` and Material components from `incular::material_prelude`.

## Contributing

Incular is actively developed and pre-1.0. Contributions and feedback are welcome; see the [contribution guide](https://github.com/FabortLabs/incular/blob/master/CONTRIBUTING.md), [code of conduct](https://github.com/FabortLabs/incular/blob/master/CODE_OF_CONDUCT.md), and [security policy](https://github.com/FabortLabs/incular/blob/master/SECURITY.md).

Licensed under [Apache-2.0](https://github.com/FabortLabs/incular/blob/master/LICENSE).
