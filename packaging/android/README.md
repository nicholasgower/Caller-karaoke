# Android packaging

Target: `aarch64-linux-android` and `armv7-linux-androideabi`, minSdk 26,
targetSdk 35.

The planned route is `cargo-apk` with the eframe Android native-activity
backend (`eframe` features `glow` + `android-native-activity`) and `cpal`'s
AAudio backend. `tts` also requires minSdk 26 for its Android callback bridge.

The app crate needs `crate-type = ["cdylib", "lib"]` and an `android_main` entry
point before `cargo apk build --release --lib` can produce the APK. This is
tracked in Milestone M5/M6 of `ROADMAP.md`; the desktop build does not require
it.

Build environment:

```sh
rustup target add aarch64-linux-android armv7-linux-androideabi
export ANDROID_HOME=...
export ANDROID_NDK_ROOT=...
cargo apk build --release --lib -p contra-app
```

Nightly CI will sign with the debug keystore unless release signing secrets are
configured.
