"""Compile supported facade feature sets; no tests or GUI applications are run."""
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def check_desktop_backends(target, expected, extra_features=()):
    command = [
        "cargo", "tree", "--locked", "-p", "incular", "--target", target,
        "-e", "normal", "-i", "incular-wgpu", "--depth", "0", "--format", "{f}",
    ]
    if extra_features:
        command += ["--features", ",".join(extra_features)]
    features = subprocess.check_output(command, cwd=ROOT, text=True).strip().split(",")
    backends = {"d3d11", "dx12", "metal", "vulkan", "gl"}.intersection(features)
    if backends != expected:
        raise RuntimeError(f"{target}: expected {expected}, resolved {backends}")


def main():
    check_desktop_backends("x86_64-pc-windows-msvc", {"d3d11"})
    check_desktop_backends("aarch64-apple-darwin", {"metal"})
    check_desktop_backends("x86_64-unknown-linux-gnu", {"vulkan"})
    check_desktop_backends("x86_64-pc-windows-msvc", {"d3d11", "vulkan", "gl"}, ("vulkan", "gl"))
    for features in ("", "desktop", "d3d11", "dx12", "metal", "vulkan", "gl", "desktop-host,vulkan,gl", "controls", "material", "devtools", "desktop,controls,material,devtools"):
        command = ["cargo", "check", "-p", "incular", "--lib", "--locked", "--no-default-features"]
        if features:
            command += ["--features", features]
        print("Checking features: " + (features or "none"), flush=True)
        subprocess.run(command, cwd=ROOT, check=True)
    subprocess.run(["cargo", "check", "-p", "incular", "--lib", "--locked"], cwd=ROOT, check=True)


if __name__ == "__main__":
    main()
