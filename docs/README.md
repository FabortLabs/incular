# Incular documentation site

The application-author guides and API reference use TanStack Start and
Fumadocs. The site source lives here; Rust API documentation is generated
separately by rustdoc and published by docs.rs after crates.io publication.

## Local development

Use Node.js 22.12 or newer and npm. From this directory:

```sh
npm ci
npm run dev
```

Open `http://localhost:3535`. Edit pages under `content/docs/`; each section's
`meta.json` controls its navigation order. Repository links are configured in
`src/lib/shared.ts`.

## Validation

```sh
npm run types:check
npm run lint
npm run build
```

The build prerenders documentation pages and emits a Vercel deployment under
`.vercel/output/` using the Nitro preset in `vite.config.ts`. Publishing the
Rust crates does not deploy this site.

From the repository root, check the Rust documentation as well:

```sh
python scripts/check_doc_examples.py --compile
cargo doc --workspace --all-features --no-deps --locked
cargo test --workspace --all-features --doc --locked
```

`check_doc_examples.py --compile` compiles every runnable MDX snippet against
the facade with default features and DevTools, rejecting snippet warnings.
It uses the locked dependencies
offline; run
`cargo fetch --locked` first on a fresh checkout. Without `--compile`, it only
checks snippet shape. Rustdoc tests cover Rust source and included crate
READMEs. Before release, rehearse the targets and features declared
in each publishable crate's `[package.metadata.docs.rs]`. A Windows workspace
documentation build alone does not establish docs.rs build success.

## docs.rs release checks

Every publishable crate declares its documentation URL, packaged README and
`[package.metadata.docs.rs]`. Check these with `python scripts/release.py metadata`.
The facade selects all features and Linux, Windows and macOS targets; native
adapter crates choose their matching target, including the Android/iOS semantic
adapters. Mobile adapters do not provide application runners.

Rehearse the packaged crates on Linux nightly, including the declared cross
targets. Check the generated pages after publishing each crate. docs.rs builds
published archives in a sandbox with read-only sources and no build-time network
access, so a checkout build is only partial evidence. See the official
[build requirements](https://docs.rs/about/builds) and
[metadata reference](https://docs.rs/about/metadata).
