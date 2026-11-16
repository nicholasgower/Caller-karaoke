#!/usr/bin/env python3
"""Build a signed Contra Karaoke APK.

`cargo apk` produces the native APK, but it cannot compile Java. The `tts`
crate's Android backend requires the `rs.tts.Bridge` class at runtime, and the
app's `MainActivity` owns the Android media session and its notification, so
this script compiles both, adds every resulting class to the APK as
`classes.dex`, re-aligns the archive, and signs it.

The release profile is signed with the debug keystore unless
`CARGO_APK_RELEASE_KEYSTORE` and `CARGO_APK_RELEASE_KEYSTORE_PASSWORD` are set,
which matches the nightly/test-build policy.
"""

from __future__ import annotations

import argparse
import os
import shutil
import subprocess
import sys
import tempfile
import zipfile
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
APP_MANIFEST = REPO / "crates" / "app" / "Cargo.toml"
BRIDGE = REPO / "packaging" / "android" / "java" / "rs" / "tts" / "Bridge.java"
MAIN_ACTIVITY = (
    REPO / "packaging" / "android" / "java" / "org" / "contrakaraoke" / "app" / "MainActivity.java"
)
TTS_PACKAGE = "rs/tts/Bridge.class"
MAIN_ACTIVITY_PACKAGE = "org/contrakaraoke/app"


def run(args: list[str], *, env: dict[str, str] | None = None, cwd: Path = REPO) -> None:
    print("+", " ".join(str(arg) for arg in args), flush=True)
    subprocess.run(args, cwd=cwd, env=env, check=True)


def output(args: list[str], *, env: dict[str, str] | None = None) -> str:
    return subprocess.check_output(args, cwd=REPO, env=env, text=True).strip()


def default_sdk() -> Path:
    for name in ("ANDROID_HOME", "ANDROID_SDK_ROOT"):
        value = os.environ.get(name)
        if value:
            return Path(value)
    home = Path.home()
    candidates = [
        Path(os.environ.get("LOCALAPPDATA", home / "AppData" / "Local")) / "Android" / "Sdk",
        home / "Android" / "Sdk",
        home / "Library" / "Android" / "sdk",
    ]
    for candidate in candidates:
        if candidate.is_dir():
            return candidate
    raise SystemExit("Android SDK not found; set ANDROID_HOME")


def default_ndk(sdk: Path) -> Path:
    for name in ("ANDROID_NDK_ROOT", "ANDROID_NDK_HOME"):
        value = os.environ.get(name)
        if value:
            return Path(value)
    ndk_root = sdk / "ndk"
    versions = sorted(
        (path for path in ndk_root.iterdir() if path.is_dir()),
        key=lambda path: path.name,
    ) if ndk_root.is_dir() else []
    if not versions:
        raise SystemExit("Android NDK not found; set ANDROID_NDK_ROOT")
    return versions[-1]


def default_java_home() -> Path:
    value = os.environ.get("JAVA_HOME")
    if value:
        return Path(value)
    home = Path.home()
    candidates = [
        Path(r"C:\Program Files\Android\Android Studio\jbr"),
        Path("/Applications/Android Studio.app/Contents/jbr/Contents/Home"),
        home / "android-studio" / "jbr",
        Path("/opt/android-studio/jbr"),
    ]
    for candidate in candidates:
        if (candidate / "bin" / ("java.exe" if os.name == "nt" else "java")).is_file():
            return candidate
    java = shutil.which("java")
    if java:
        return Path(java).resolve().parent.parent
    raise SystemExit("JDK not found; set JAVA_HOME")


def build_tool(sdk: Path, name: str) -> Path:
    build_tools = sdk / "build-tools"
    versions = sorted(
        (path for path in build_tools.iterdir() if path.is_dir()),
        key=lambda path: tuple(int(part) if part.isdigit() else part for part in path.name.split(".")),
    ) if build_tools.is_dir() else []
    if not versions:
        raise SystemExit(f"{build_tools} has no build-tools")
    suffix = ".bat" if os.name == "nt" and name in {"apksigner", "d8"} else ""
    return versions[-1] / f"{name}{suffix}"


def android_jar(sdk: Path) -> Path:
    platforms = sdk / "platforms"
    candidates = sorted(
        (path for path in platforms.iterdir() if path.is_dir()),
        key=lambda path: path.name,
    ) if platforms.is_dir() else []
    for platform in reversed(candidates):
        jar = platform / "android.jar"
        if jar.is_file():
            return jar
    raise SystemExit("Android platform android.jar not found")


def tool_env(sdk: Path, ndk: Path, java_home: Path) -> dict[str, str]:
    env = os.environ.copy()
    env["ANDROID_HOME"] = str(sdk)
    env["ANDROID_SDK_ROOT"] = str(sdk)
    env["ANDROID_NDK_ROOT"] = str(ndk)
    env["JAVA_HOME"] = str(java_home)
    # Keep cargo-apk and cargo on the same target directory. A user-level
    # build.target-dir would otherwise redirect cargo but not cargo-apk.
    env["CARGO_TARGET_DIR"] = str(REPO / "target")
    path = [str(java_home / "bin"), str(sdk / "platform-tools")]
    path.extend(
        str(path)
        for path in (
            sdk / "cmdline-tools" / "latest" / "bin",
            sdk / "build-tools" / build_tool(sdk, "aapt").parent.name,
        )
        if path.is_dir()
    )
    if env.get("PATH"):
        path.append(env["PATH"])
    env["PATH"] = os.pathsep.join(path)
    return env


def debug_keystore(env: dict[str, str], java_home: Path) -> Path:
    path = Path.home() / ".android" / "debug.keystore"
    if path.is_file():
        return path
    path.parent.mkdir(parents=True, exist_ok=True)
    keytool = java_home / "bin" / ("keytool.exe" if os.name == "nt" else "keytool")
    run(
        [
            str(keytool),
            "-genkeypair",
            "-v",
            "-keystore",
            str(path),
            "-storepass",
            "android",
            "-alias",
            "androiddebugkey",
            "-keypass",
            "android",
            "-dname",
            "CN=Android Debug,O=Android,C=US",
            "-keyalg",
            "RSA",
            "-keysize",
            "2048",
            "-validity",
            "10000",
        ],
        env=env,
    )
    return path


def build_rust(profile: str, target: str | None, env: dict[str, str], keystore: Path | None) -> None:
    args = ["cargo", "apk", "build", "--lib", "-p", "contra-app"]
    if profile == "release":
        args.append("--release")
    if target:
        args.extend(["--target", target])
    if profile == "release" and keystore:
        env = dict(env)
        env.setdefault("CARGO_APK_RELEASE_KEYSTORE", str(keystore))
        env.setdefault("CARGO_APK_RELEASE_KEYSTORE_PASSWORD", "android")
    run(args, env=env)


def build_dex(env: dict[str, str], sdk: Path, java_home: Path, work: Path) -> Path:
    classes = work / "classes"
    dex = work / "dex"
    classes.mkdir(parents=True)
    dex.mkdir(parents=True)
    javac = java_home / "bin" / ("javac.exe" if os.name == "nt" else "javac")
    jar = android_jar(sdk)
    run(
        [
            str(javac),
            "-source",
            "8",
            "-target",
            "8",
            "-Xlint:-options",
            "-classpath",
            str(jar),
            "-d",
            str(classes),
            str(BRIDGE),
            str(MAIN_ACTIVITY),
        ],
        env=env,
    )
    class_files = [classes / TTS_PACKAGE]
    # The media session's callback and notification receiver are anonymous
    # classes, which javac emits as MainActivity$1.class and friends. Missing
    # one of them makes the session fail with NoClassDefFoundError at runtime,
    # so every class the activity compiles to is packaged.
    app_classes = sorted((classes / MAIN_ACTIVITY_PACKAGE).glob("MainActivity*.class"))
    for class_file in [*class_files, *app_classes]:
        if not class_file.is_file():
            raise SystemExit(f"{class_file} was not produced")
    class_files.extend(app_classes)
    run(
        [
            str(build_tool(sdk, "d8")),
            "--lib",
            str(jar),
            "--output",
            str(dex),
            *[str(class_file) for class_file in class_files],
        ],
        env=env,
    )
    classes_dex = dex / "classes.dex"
    if not classes_dex.is_file():
        raise SystemExit(f"{classes_dex} was not produced")
    return classes_dex


def inject_dex(apk: Path, classes_dex: Path, output: Path) -> None:
    with zipfile.ZipFile(apk) as source, zipfile.ZipFile(
        output, "w", compression=zipfile.ZIP_DEFLATED
    ) as target:
        for item in source.infolist():
            if item.filename.startswith("META-INF/") or item.filename == "classes.dex":
                continue
            target.writestr(item, source.read(item.filename))
        target.write(classes_dex, "classes.dex")


def sign(
    env: dict[str, str],
    sdk: Path,
    unsigned: Path,
    keystore: Path,
    password: str,
    key_password: str,
    output: Path,
) -> None:
    aligned = output.with_name(output.stem + "-aligned.apk")
    run(
        [
            str(build_tool(sdk, "zipalign")),
            "-f",
            "-p",
            "4",
            str(unsigned),
            str(aligned),
        ],
        env=env,
    )
    run(
        [
            str(build_tool(sdk, "apksigner")),
            "sign",
            "--ks",
            str(keystore),
            "--ks-pass",
            f"pass:{password}",
            "--key-pass",
            f"pass:{key_password}",
            "--in",
            str(aligned),
            "--out",
            str(output),
        ],
        env=env,
    )
    run(
        [str(build_tool(sdk, "apksigner")), "verify", "--print-certs", str(output)],
        env=env,
    )


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--debug", action="store_true", help="build the dev profile")
    parser.add_argument("--target", help="build one Rust target, e.g. x86_64-linux-android")
    parser.add_argument("--skip-build", action="store_true", help="reuse the existing APK")
    parser.add_argument("--output", type=Path, help="final APK path")
    args = parser.parse_args()

    profile = "debug" if args.debug else "release"
    sdk = default_sdk()
    ndk = default_ndk(sdk)
    java_home = default_java_home()
    env = tool_env(sdk, ndk, java_home)

    keystore_env = os.environ.get("CARGO_APK_RELEASE_KEYSTORE")
    keystore = Path(keystore_env) if keystore_env else debug_keystore(env, java_home)
    password = os.environ.get("CARGO_APK_RELEASE_KEYSTORE_PASSWORD", "android")
    key_password = os.environ.get("CARGO_APK_RELEASE_KEY_PASSWORD", password)

    if not args.skip_build:
        build_rust(profile, args.target, env, None if args.debug else keystore)

    apk = REPO / "target" / profile / "apk" / "contra-karaoke.apk"
    if not apk.is_file():
        raise SystemExit(f"{apk} was not produced")

    work = Path(tempfile.mkdtemp(prefix="contra-android-", dir=REPO / "target"))
    try:
        classes_dex = build_dex(env, sdk, java_home, work)
        unsigned = work / "contra-karaoke-unsigned.apk"
        inject_dex(apk, classes_dex, unsigned)
        final = args.output or apk
        final.parent.mkdir(parents=True, exist_ok=True)
        sign(env, sdk, unsigned, keystore, password, key_password, final)
        print(f"\nAPK: {final} ({final.stat().st_size:,} bytes)")
    finally:
        shutil.rmtree(work, ignore_errors=True)
    return 0


if __name__ == "__main__":
    sys.exit(main())
