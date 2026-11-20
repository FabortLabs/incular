//! `incular`: builds, installs and screenshots Incular apps on Android and
//! iOS.
//!
//! ```text
//! incular build android --example counter [--release] [--abi x86_64]
//! incular build ios     --example counter [--release] [--device]
//! incular run   android --example counter [--release] [--abi x86_64] [--screenshot out.png] [--wait 8]
//! incular run   ios     --example counter [--release] [--screenshot out.png] [--wait 8]
//! ```
//!
//! `--bin <name>` builds an application binary instead of an example, and
//! `-p/--package` selects the Cargo package (default `incular` for
//! examples). The tool drives Cargo, Gradle and Xcode's command-line tools; it
//! never replaces them. See `system-design/MOBILE.md`.

mod android;
mod ios;
mod process;

use std::{path::PathBuf, process::ExitCode};

/// What to build, parsed from the command line.
#[derive(Clone, Debug)]
pub(crate) struct Options {
    pub(crate) package: String,
    pub(crate) target: Target,
    pub(crate) release: bool,
    pub(crate) abi: String,
    pub(crate) device: bool,
    pub(crate) features: String,
    pub(crate) screenshot: Option<PathBuf>,
    pub(crate) wait_seconds: u64,
    pub(crate) out_dir: PathBuf,
}

/// The Cargo target that holds the app's `main`.
#[derive(Clone, Debug)]
pub(crate) enum Target {
    Example(String),
    Bin(String),
}

impl Target {
    pub(crate) fn name(&self) -> &str {
        match self {
            Self::Example(name) | Self::Bin(name) => name,
        }
    }

    pub(crate) fn cargo_args(&self) -> [&str; 2] {
        match self {
            Self::Example(name) => ["--example", name],
            Self::Bin(name) => ["--bin", name],
        }
    }

    /// The directory Cargo writes this target's executable to, relative to
    /// the profile directory.
    pub(crate) fn output_subdir(&self) -> &'static str {
        match self {
            Self::Example(_) => "examples",
            Self::Bin(_) => "",
        }
    }
}

const USAGE: &str = "\
usage: incular <build|run> <android|ios> (--example <name> | --bin <name>) [options]

options:
  -p, --package <name>     Cargo package (default: incular)
      --release            build with the release profile
      --abi <abi>          Android ABI: arm64-v8a (default) or x86_64
      --device             iOS: build for a device instead of the simulator
      --features <list>    Cargo features (default: mobile,controls,material)
      --screenshot <path>  run: save a screenshot after --wait seconds
      --wait <seconds>     run: seconds to wait before the screenshot (default 8)
      --out <dir>          output directory (default: target/incular)";

fn parse(arguments: &[String]) -> Result<(String, String, Options), String> {
    let mut iter = arguments.iter();
    let command = iter.next().ok_or(USAGE)?.clone();
    let platform = iter.next().ok_or(USAGE)?.clone();
    let mut target = None;
    let mut options = Options {
        package: "incular".into(),
        target: Target::Example(String::new()),
        release: false,
        abi: "arm64-v8a".into(),
        device: false,
        features: "mobile,controls,material".into(),
        screenshot: None,
        wait_seconds: 8,
        out_dir: PathBuf::from("target").join("incular"),
    };
    while let Some(argument) = iter.next() {
        let mut value = || {
            iter.next()
                .cloned()
                .ok_or_else(|| format!("{argument} needs a value"))
        };
        match argument.as_str() {
            "--example" => target = Some(Target::Example(value()?)),
            "--bin" => target = Some(Target::Bin(value()?)),
            "-p" | "--package" => options.package = value()?,
            "--release" => options.release = true,
            "--abi" => options.abi = value()?,
            "--device" => options.device = true,
            "--features" => options.features = value()?,
            "--screenshot" => options.screenshot = Some(PathBuf::from(value()?)),
            "--wait" => {
                options.wait_seconds = value()?
                    .parse()
                    .map_err(|_| "--wait takes a number of seconds")?;
            }
            "--out" => options.out_dir = PathBuf::from(value()?),
            "-h" | "--help" => return Err(USAGE.into()),
            other => return Err(format!("unknown argument {other}\n\n{USAGE}")),
        }
    }
    options.target = target.ok_or("pass --example <name> or --bin <name>")?;
    Ok((command, platform, options))
}

fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let (command, platform, options) = match parse(&arguments) {
        Ok(parsed) => parsed,
        Err(message) => {
            eprintln!("{message}");
            return ExitCode::FAILURE;
        }
    };
    let result = match (command.as_str(), platform.as_str()) {
        ("build", "android") => android::build(&options).map(|apk| {
            println!("{}", apk.display());
        }),
        ("run", "android") => android::build(&options).and_then(|apk| android::run(&options, &apk)),
        ("build", "ios") => ios::build(&options).map(|app| {
            println!("{}", app.display());
        }),
        ("run", "ios") => ios::build(&options).and_then(|app| ios::run(&options, &app)),
        _ => Err(USAGE.into()),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("incular: {message}");
            ExitCode::FAILURE
        }
    }
}
