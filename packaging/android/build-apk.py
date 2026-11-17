#!/usr/bin/env python3
"""Build a signed Caller Karaoke APK.

`cargo apk` produces the native APK, but it cannot compile Java. The `tts`
crate's Android backend requires the `rs.tts.Bridge` class at runtime, and the
app's `MainActivity` owns the Android media session and its notification, so
this script compiles both, adds every resulting class to the APK as
`classes.dex`, adds the pinned ONNX Runtime library for the local Kokoro voice,
re-aligns the archive, and signs it.

The release profile is signed with the debug keystore unless
`CARGO_APK_RELEASE_KEYSTORE` and `CARGO_APK_RELEASE_KEYSTORE_PASSWORD` are set,
which matches the test-build policy.
"""

from __future__ import annotations

import argparse
import hashlib
import os
import shutil
import subprocess
import sys
import tempfile
import urllib.request
import zipfile
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
APP_MANIFEST = REPO / "crates" / "app" / "Cargo.toml"
BRIDGE = REPO / "packaging" / "android" / "java" / "rs" / "tts" / "Bridge.java"
MAIN_ACTIVITY = (
    REPO / "packaging" / "android" / "java" / "org" / "callerkaraoke" / "app" / "MainActivity.java"
)
TTS_PACKAGE = "rs/tts/Bridge.class"
MAIN_ACTIVITY_PACKAGE = "org/callerkaraoke/app"

# ONNX Runtime for the local Kokoro voice. Only libonnxruntime.so is taken from
# the AAR; the Java bindings and the JNI wrapper are not used.
ORT_AAR_VERSION = "1.31.0"
ORT_AAR_NAME = f"onnxruntime-android-{ORT_AAR_VERSION}.aar"
ORT_AAR_URL = (
    "https://repo1.maven.org/maven2/com/microsoft/onnxruntime/"
    f"onnxruntime-android/{ORT_AAR_VERSION}/{ORT_AAR_NAME}"
)
ORT_AAR_SHA256 = "72683787a80d0608a3121b967d8e1239aef48286826661c953211626690540a7"
ORT_LIBRARY = "libonnxruntime.so"


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
    args = ["cargo", "apk", "build", "--lib", "-p", "caller-app"]
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


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as file:
        for chunk in iter(lambda: file.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def find_ort_aar() -> Path:
    override = os.environ.get("CALLER_ORT_ANDROID_AAR")
    if override:
        aar = Path(override)
        if not aar.is_file():
            raise SystemExit(f"CALLER_ORT_ANDROID_AAR does not exist: {aar}")
        if sha256_file(aar) != ORT_AAR_SHA256:
            raise SystemExit(
                "CALLER_ORT_ANDROID_AAR does not match the pinned ONNX Runtime "
                f"{ORT_AAR_VERSION} AAR"
            )
        return aar

    cache = REPO / "target" / "android-ort" / ORT_AAR_NAME
    if cache.is_file() and sha256_file(cache) == ORT_AAR_SHA256:
        return cache

    cache.parent.mkdir(parents=True, exist_ok=True)
    partial = cache.with_suffix(".aar.part")
    print(f"+ download {ORT_AAR_URL}", flush=True)
    request = urllib.request.Request(
        ORT_AAR_URL, headers={"User-Agent": "caller-karaoke-build"}
    )
    with urllib.request.urlopen(request, timeout=300) as response, partial.open(
        "wb"
    ) as output:
        shutil.copyfileobj(response, output, length=1024 * 1024)
    if sha256_file(partial) != ORT_AAR_SHA256:
        partial.unlink(missing_ok=True)
        raise SystemExit(
            f"ONNX Runtime {ORT_AAR_VERSION} download did not match the pinned SHA-256"
        )
    partial.replace(cache)
    return cache


def apk_native_abis(apk: Path) -> list[str]:
    with zipfile.ZipFile(apk) as archive:
        return sorted(
            {
                name.split("/")[1]
                for name in archive.namelist()
                if name.startswith("lib/")
                and name.endswith("/libcaller_app.so")
                and len(name.split("/")) == 3
            }
        )


def inject_assets(
    apk: Path,
    classes_dex: Path,
    output: Path,
    *,
    ort_aar: Path | None,
) -> None:
    abis = apk_native_abis(apk)
    if not abis:
        raise SystemExit(f"{apk} has no lib/<abi>/libcaller_app.so entries")
    ort_libraries: dict[str, str] = {}
    if ort_aar is not None:
        with zipfile.ZipFile(ort_aar) as runtime:
            runtime_names = set(runtime.namelist())
        for abi in abis:
            entry = f"jni/{abi}/{ORT_LIBRARY}"
            if entry not in runtime_names:
                raise SystemExit(
                    f"ONNX Runtime {ORT_AAR_VERSION} has no {ORT_LIBRARY} for {abi}"
                )
            ort_libraries[abi] = entry
    else:
        print("warning: building without the Kokoro neural runtime", flush=True)

    with zipfile.ZipFile(apk) as source, zipfile.ZipFile(
        output, "w", compression=zipfile.ZIP_DEFLATED
    ) as target:
        for item in source.infolist():
            if item.filename.startswith("META-INF/") or item.filename == "classes.dex":
                continue
            if item.filename.startswith("lib/") and item.filename.endswith(
                f"/{ORT_LIBRARY}"
            ):
                continue
            target.writestr(item, source.read(item.filename))
        target.write(classes_dex, "classes.dex")

        if ort_aar is not None:
            with zipfile.ZipFile(ort_aar) as runtime:
                for abi, entry in sorted(ort_libraries.items()):
                    info = zipfile.ZipInfo(f"lib/{abi}/{ORT_LIBRARY}")
                    info.compress_type = zipfile.ZIP_DEFLATED
                    info.external_attr = 0o644 << 16
                    with runtime.open(entry) as source_file, target.open(info, "w") as output_file:
                        shutil.copyfileobj(source_file, output_file)
                    print(f"+ {info.filename}", flush=True)


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
            # 16 KB is the page size Android 15 and newer require for shared
            # libraries. The ONNX Runtime binaries from the AAR are built for it.
            "-P",
            "16",
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
    parser.add_argument(
        "--without-kokoro",
        action="store_true",
        help="omit the ONNX Runtime library and use the platform voice",
    )
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

    apk = REPO / "target" / profile / "apk" / "caller-karaoke.apk"
    if not apk.is_file():
        raise SystemExit(f"{apk} was not produced")

    work = Path(tempfile.mkdtemp(prefix="caller-android-", dir=REPO / "target"))
    try:
        classes_dex = build_dex(env, sdk, java_home, work)
        ort_aar = None if args.without_kokoro else find_ort_aar()
        unsigned = work / "caller-karaoke-unsigned.apk"
        inject_assets(apk, classes_dex, unsigned, ort_aar=ort_aar)
        final = args.output or apk
        final.parent.mkdir(parents=True, exist_ok=True)
        sign(env, sdk, unsigned, keystore, password, key_password, final)
        print(f"\nAPK: {final} ({final.stat().st_size:,} bytes)")
    finally:
        shutil.rmtree(work, ignore_errors=True)
    return 0


if __name__ == "__main__":
    sys.exit(main())
