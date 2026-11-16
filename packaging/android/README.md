# Android packaging

Target: `aarch64-linux-android` and `armv7-linux-androideabi`, minSdk 26,
targetSdk 35, package `org.contrakaraoke.app`.

The build uses `cargo-apk` with the eframe Android native-activity backend
(`eframe` features `glow` + `android-native-activity`) and `cpal`'s AAudio
backend.

## Build

Install the SDK platform, build-tools, NDK, and Rust targets:

```sh
rustup target add --toolchain 1.100.0 \
  aarch64-linux-android armv7-linux-androideabi x86_64-linux-android
sdkmanager "platform-tools" "platforms;android-35" \
  "build-tools;35.0.0" "ndk;27.0.12077973"
```

Then run the packaging script. It finds the SDK/NDK/JDK from the usual
environment variables or Android Studio's default locations, builds the native
APK, compiles the TTS Java bridge and the `MainActivity` that loads the native
library from the app class loader, injects both as `classes.dex`, aligns the
archive, and signs it:

```sh
python packaging/android/build-apk.py
```

The result is `target/release/apk/contra-karaoke.apk`. Add `--debug` for a
debug build. `--target x86_64-linux-android` builds a single-ABI APK for an
x86_64 emulator.

The custom activity matters: `tts`'s `JNI_OnLoad` calls
`FindClass("rs/tts/Bridge")`, and loading the library from the framework's
`NativeActivity` would use a class loader that cannot see the app's
`classes.dex`.

Do not use bare `cargo apk build` for a distributable artifact: cargo-apk
cannot compile the `rs.tts.Bridge` class that the `tts` crate's Android backend
requires, and it produces an APK with an empty `classes.dex` slot. The script
handles that step, sets `android:hasCode="true"`, removes the old signature,
and signs the final archive.

## Signing

The script uses `CARGO_APK_RELEASE_KEYSTORE` and
`CARGO_APK_RELEASE_KEYSTORE_PASSWORD` when they are set. Otherwise it signs the
release build with the local Android debug keystore (`~/.android/debug.keystore`)
and clearly labels the result as a test build. Release signing secrets belong in
CI, never in the repository.

Do not set a user-level `build.target-dir` and expect `cargo apk` to find the
shared libraries; the script pins `CARGO_TARGET_DIR` to the workspace `target/`
so cargo and cargo-apk agree.

## Launcher icons

The launcher icons for all five densities are in `res/mipmap-*`
(`ic_launcher.png`, 48 to 192 px). They are legacy icons, not an adaptive
foreground/background pair; moving to adaptive icons means shrinking the art
into the 66% safe circle first.
