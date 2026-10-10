# incular-hot-reload-example

A small counter to edit while it runs. Start it with the
[Dioxus CLI](https://dioxuslabs.com/learn/0.7/getting_started/), naming your
desktop platform (`--windows`, `--macos` or `--linux`):

```text
dx serve --hot-patch --windows -p incular-hot-reload-example
```

Click Increment a few times, then change anything marked "try editing" in
`main.rs` and save. The window updates in place and the count is kept.

`cargo run -p incular-hot-reload-example` runs the same app without live
patching.

## Why this example is its own package

The other examples are targets of the `incular` package. The build driver
patches only the package it serves and watches only that package's directory,
so an application that should hot reload has to be a package of its own that no
other workspace crate depends on. This is also the shape of a real application.

## Architecture and support

| Contract | Status |
| --- | --- |
| Ownership | Standalone hot reload example application over the public facade. |
| API class | application; this package is an example, not a framework re-export surface. |
| Support | Development example; live patching needs the Dioxus CLI and a debug build. |
