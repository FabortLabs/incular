//! Android: cross-compile the app binary as a shared object, generate a
//! Gradle project around the Kotlin shell, and build an APK.

use crate::{Options, process};
use std::{
    path::{Path, PathBuf},
    process::Command,
    time::Duration,
};

const COMPILE_SDK: u32 = 35;
const TARGET_SDK: u32 = 35;
const MIN_SDK: u32 = 26;
const AGP_VERSION: &str = "8.7.3";
const KOTLIN_VERSION: &str = "2.1.0";

struct Abi {
    triple: &'static str,
    clang_prefix: &'static str,
}

fn abi(name: &str) -> Result<Abi, String> {
    match name {
        "arm64-v8a" => Ok(Abi {
            triple: "aarch64-linux-android",
            clang_prefix: "aarch64-linux-android",
        }),
        "x86_64" => Ok(Abi {
            triple: "x86_64-linux-android",
            clang_prefix: "x86_64-linux-android",
        }),
        "armeabi-v7a" => Ok(Abi {
            triple: "armv7-linux-androideabi",
            clang_prefix: "armv7a-linux-androideabi",
        }),
        other => Err(format!(
            "unsupported ABI {other}; use arm64-v8a, x86_64 or armeabi-v7a"
        )),
    }
}

fn sdk_dir() -> Result<PathBuf, String> {
    ["ANDROID_HOME", "ANDROID_SDK_ROOT"]
        .iter()
        .find_map(std::env::var_os)
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("LOCALAPPDATA")
                .map(|local| PathBuf::from(local).join("Android").join("Sdk"))
                .filter(|path| path.is_dir())
        })
        .ok_or_else(|| "set ANDROID_HOME to the Android SDK".into())
}

fn ndk_dir(sdk: &Path) -> Result<PathBuf, String> {
    if let Some(ndk) = [
        "ANDROID_NDK_HOME",
        "ANDROID_NDK_ROOT",
        "ANDROID_NDK_LATEST_HOME",
    ]
    .iter()
    .find_map(std::env::var_os)
    {
        return Ok(PathBuf::from(ndk));
    }
    let mut versions: Vec<PathBuf> = std::fs::read_dir(sdk.join("ndk"))
        .map_err(|_| "no NDK found; install one with sdkmanager or set ANDROID_NDK_HOME")?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .collect();
    versions.sort();
    versions
        .pop()
        .ok_or_else(|| "no NDK found under $ANDROID_HOME/ndk".into())
}

fn host_tag() -> &'static str {
    if cfg!(windows) {
        "windows-x86_64"
    } else if cfg!(target_os = "macos") {
        "darwin-x86_64"
    } else {
        "linux-x86_64"
    }
}

/// Cross-compiles the app's binary into a shared object. Its C `main` runs
/// the app's `main` when the activity starts (see `incular-android`).
fn build_library(options: &Options, abi: &Abi, ndk: &Path) -> Result<PathBuf, String> {
    let bin = ndk
        .join("toolchains")
        .join("llvm")
        .join("prebuilt")
        .join(host_tag())
        .join("bin");
    let script = if cfg!(windows) { ".cmd" } else { "" };
    let clang = bin.join(format!("{}{MIN_SDK}-clang{script}", abi.clang_prefix));
    let clangxx = bin.join(format!("{}{MIN_SDK}-clang++{script}", abi.clang_prefix));
    let ar = bin.join(if cfg!(windows) {
        "llvm-ar.exe"
    } else {
        "llvm-ar"
    });
    let env_triple = abi.triple.replace('-', "_");
    let mut command = Command::new("cargo");
    command
        .args(["rustc", "-p", &options.package])
        .args(options.target.cargo_args())
        .args([
            "--target",
            abi.triple,
            "--no-default-features",
            "--features",
            &options.features,
        ])
        .env(format!("CC_{env_triple}"), &clang)
        .env(format!("CXX_{env_triple}"), &clangxx)
        .env(format!("AR_{env_triple}"), &ar)
        .env(
            format!("CARGO_TARGET_{}_LINKER", env_triple.to_uppercase()),
            &clang,
        );
    if options.release {
        command.arg("--release");
    }
    // Link the binary as a shared object the activity can load, keep the
    // JNI entry point, and leave debug info out of the APK.
    command.args([
        "--",
        "-C",
        "link-arg=-shared",
        "-C",
        "link-arg=-Wl,-u,JNI_OnLoad",
        "-C",
        "strip=debuginfo",
    ]);
    process::run(&mut command)?;
    let metadata = process::cargo_metadata()?;
    let profile = if options.release { "release" } else { "debug" };
    Ok(process::target_dir(&metadata)
        .join(abi.triple)
        .join(profile)
        .join(options.target.output_subdir())
        .join(options.target.name()))
}

/// Builds a debug-signed APK and returns its path.
pub(crate) fn build(options: &Options) -> Result<PathBuf, String> {
    let abi = abi(&options.abi)?;
    let sdk = sdk_dir()?;
    let ndk = ndk_dir(&sdk)?;
    let library = build_library(options, &abi, &ndk)?;
    let metadata = process::cargo_metadata()?;
    let shell = process::package_dir(&metadata, "incular-android")?
        .join("android")
        .join("src")
        .join("main")
        .join("kotlin");
    let name = options.target.name();
    let project = std::path::absolute(options.out_dir.join("android").join(name))
        .map_err(|error| format!("bad output directory: {error}"))?;
    let jni_libs = project.join("app").join("jniLibs").join(&options.abi);
    std::fs::create_dir_all(&jni_libs).map_err(|error| error.to_string())?;
    std::fs::copy(&library, jni_libs.join("libincular_app.so"))
        .map_err(|error| format!("cannot copy {}: {error}", library.display()))?;
    write_project(&project, name, &shell, &sdk)?;
    let gradle = std::env::var("INCULAR_GRADLE").unwrap_or_else(|_| {
        if cfg!(windows) {
            "gradle.bat"
        } else {
            "gradle"
        }
        .into()
    });
    process::run(
        Command::new(gradle)
            .arg("-p")
            .arg(&project)
            .args(["assembleDebug", "--quiet"]),
    )?;
    Ok(project
        .join("app")
        .join("build")
        .join("outputs")
        .join("apk")
        .join("debug")
        .join("app-debug.apk"))
}

fn application_id(name: &str) -> String {
    format!("dev.incular.apps.{}", process::identifier(name))
}

fn write_project(project: &Path, name: &str, shell: &Path, sdk: &Path) -> Result<(), String> {
    let id = application_id(name);
    process::write(
        &project.join("local.properties"),
        format!("sdk.dir={}\n", process::portable(sdk)),
    )?;
    process::write(
        &project.join("gradle.properties"),
        "org.gradle.jvmargs=-Xmx4g\nandroid.useAndroidX=false\nkotlin.code.style=official\n",
    )?;
    process::write(
        &project.join("settings.gradle.kts"),
        format!(
            r#"pluginManagement {{
    repositories {{ google(); mavenCentral(); gradlePluginPortal() }}
}}
dependencyResolutionManagement {{
    repositories {{ google(); mavenCentral() }}
}}
rootProject.name = "{name}"
include(":app")
"#
        ),
    )?;
    process::write(
        &project.join("build.gradle.kts"),
        format!(
            r#"plugins {{
    id("com.android.application") version "{AGP_VERSION}" apply false
    id("org.jetbrains.kotlin.android") version "{KOTLIN_VERSION}" apply false
}}
"#
        ),
    )?;
    process::write(
        &project.join("app").join("build.gradle.kts"),
        format!(
            r#"// Generated by `incular`; edits are overwritten on the next build.
plugins {{
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
}}

android {{
    namespace = "{id}"
    compileSdk = {COMPILE_SDK}
    defaultConfig {{
        applicationId = "{id}"
        minSdk = {MIN_SDK}
        targetSdk = {TARGET_SDK}
        versionCode = 1
        versionName = "1.0"
    }}
    sourceSets.getByName("main") {{
        java.srcDirs("{shell}")
        jniLibs.srcDirs("jniLibs")
    }}
    compileOptions {{
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }}
    kotlinOptions {{ jvmTarget = "17" }}
    packaging {{ jniLibs {{ useLegacyPackaging = false }} }}
}}
"#,
            shell = process::portable(shell),
        ),
    )?;
    process::write(
        &project
            .join("app")
            .join("src")
            .join("main")
            .join("AndroidManifest.xml"),
        format!(
            r#"<?xml version="1.0" encoding="utf-8"?>
<!-- Generated by `incular`; edits are overwritten on the next build. -->
<manifest xmlns:android="http://schemas.android.com/apk/res/android">
    <uses-permission android:name="android.permission.INTERNET" />
    <application
        android:label="{name}"
        android:hasCode="true"
        android:enableOnBackInvokedCallback="true">
        <activity
            android:name="dev.incular.IncularActivity"
            android:exported="true"
            android:launchMode="singleTop"
            android:hardwareAccelerated="true"
            android:windowSoftInputMode="adjustResize"
            android:configChanges="orientation|screenSize|smallestScreenSize|screenLayout|density|uiMode|keyboard|keyboardHidden|navigation|fontScale|layoutDirection|locale"
            android:theme="@android:style/Theme.Material.Light.NoActionBar">
            <meta-data android:name="dev.incular.lib_name" android:value="incular_app" />
            <intent-filter>
                <action android:name="android.intent.action.MAIN" />
                <category android:name="android.intent.category.LAUNCHER" />
            </intent-filter>
        </activity>
    </application>
</manifest>
"#
        ),
    )
}

/// Installs and launches the APK, then optionally saves a screenshot and the
/// app's log next to it.
pub(crate) fn run(options: &Options, apk: &Path) -> Result<(), String> {
    let adb = sdk_dir()?
        .join("platform-tools")
        .join(if cfg!(windows) { "adb.exe" } else { "adb" });
    let id = application_id(options.target.name());
    process::run(Command::new(&adb).arg("install").arg("-r").arg(apk))?;
    process::run(Command::new(&adb).args(["logcat", "-c"]))?;
    process::run(Command::new(&adb).args([
        "shell",
        "am",
        "start",
        "-W",
        "-n",
        &format!("{id}/dev.incular.IncularActivity"),
    ]))?;
    let Some(screenshot) = &options.screenshot else {
        return Ok(());
    };
    std::thread::sleep(Duration::from_secs(options.wait_seconds));
    let image = process::output(Command::new(&adb).args(["exec-out", "screencap", "-p"]))?;
    process::write(screenshot, image.stdout)?;
    let log = process::output(Command::new(&adb).args([
        "logcat",
        "-d",
        "-s",
        "Incular:*",
        "AndroidRuntime:*",
        "DEBUG:*",
        "libc:*",
    ]))?;
    process::write(&screenshot.with_extension("log"), log.stdout)?;
    let _ = Command::new(&adb)
        .args(["shell", "am", "force-stop", &id])
        .status();
    Ok(())
}
