# Android packaging

Target: `aarch64-linux-android` and `armv7-linux-androideabi`, minSdk 26,
targetSdk 35, package `org.callerkaraoke.app`.

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
APK, downloads the pinned ONNX Runtime Android AAR, adds its
`libonnxruntime.so` to the APK, compiles the TTS Java bridge and the
`MainActivity` that loads the native library from the app class loader, injects
both as `classes.dex`, aligns the archive, and signs it:

```sh
python packaging/android/build-apk.py
```

The result is `target/release/apk/caller-karaoke.apk`. Add `--debug` for a
debug build. `--target x86_64-linux-android` builds a single-ABI APK for an
x86_64 emulator.

The custom activity matters: `tts`'s `JNI_OnLoad` calls
`FindClass("rs/tts/Bridge")`, and loading the library from the framework's
`NativeActivity` would use a class loader that cannot see the app's
`classes.dex`.

## Neural voice runtime

The Android build uses the same local Kokoro voice as the desktop build. ONNX
Runtime is not linked into the Rust library; `libonnxruntime.so` is loaded from
the app's native library directory at runtime. `build-apk.py` downloads the
pinned `onnxruntime-android` 1.31.0 AAR from Maven Central, verifies its
SHA-256, and adds the AAR's `jni/<abi>/libonnxruntime.so` for each ABI present
in the APK. The APK sets `android:extractNativeLibs="true"`, so the library is
extracted to the app's native library directory on install. Both the app library
and the packaged ONNX Runtime binary use 16 KB page alignment for Android 15
and newer.

The AAR is cached under `target/android-ort/`; it is not committed. For an
offline build, point `CALLER_ORT_ANDROID_AAR` at the pinned AAR. Pass
`--without-kokoro` to build an APK with only the platform TTS backend, which is
useful for testing the fallback or reducing the APK size.

The model and voice style files are still fetched by the app on first use and
stored in the app's cache; they are not packaged in the APK. Adding the runtime
makes the APK larger because the library is compressed inside the archive and
extracted at install time.

Do not use bare `cargo apk build` for a distributable artifact: cargo-apk
cannot compile the `rs.tts.Bridge` class that the `tts` crate's Android backend
requires, and it produces an APK with an empty `classes.dex` slot. The script
handles that step, sets `android:hasCode="true"`, removes the old signature,
and signs the final archive.

## Media session

`MainActivity` registers an `android.media.session.MediaSession` in `onCreate`
and posts a `MediaStyle` notification whose title is the loaded dance, whose
artist is the tune, and whose album is *Caller Karaoke*. Headset, Bluetooth,
lock-screen, notification, and system media controls arrive at the session's
`MediaSession.Callback` (and at a notification `BroadcastReceiver` on Android 12
and older), which forwards them to Rust through the static native method
`nativeMediaCommand(int)`.

Rust publishes back through `setMediaPlaybackState(boolean)` and
`setMediaMetadata(String, String, String)` on the same class; those calls come
from `crates/app/src/media_keys.rs`. The command codes are shared between
`MainActivity.java` (`COMMAND_*`) and `media_keys.rs` (`android_bridge`), so a
change on one side must change the other.

`MainActivity` calls `nativeAttachActivity()` in `onCreate`, and Rust keeps the
class as a global reference. The event loop runs on a native thread attached
without the app class loader, so `FindClass` there raises
`ClassNotFoundException` for `org.callerkaraoke.app.MainActivity`, and the
pending exception aborts the process when the thread detaches. Every later
media call goes through the handle the activity published instead.

The callback and the broadcast receiver are anonymous classes, so `javac`
emits `MainActivity$1.class`, `MainActivity$2.class`, and so on. `build-apk.py`
packages every `MainActivity*.class` into `classes.dex`; packaging only the
activity itself makes the session fail with `NoClassDefFoundError`.

`POST_NOTIFICATIONS` is declared in `crates/app/Cargo.toml` and requested at
startup on Android 13+. Declining only hides the notification: the media session
is independent of it, so headset, Bluetooth, lock-screen, and system controls
still work. Nothing here is a foreground service; Android may still stop the app
in the background, and playback stops with it.

## Text input

A native activity's content view is a plain `View` with no input connection, so
Android's `InputMethodManager.showSoftInput()` ignores it and no keyboard
appears. `MainActivity` therefore adds a one-pixel, fully transparent `EditText`
to the window and focuses it when egui has a text field focused.

- `setTextInputActive(boolean)` is called from Rust through the same activity
  bridge as the media session. It requests focus and shows the IME, or hides the
  IME and clears focus.
- The bridge is the **only** Android IME controller. `egui` publishes its own IME
  request, and `winit` would turn that into a second
  `show_soft_input`/`hide_soft_input` pair on the decor view. Rust clears that
  output on Android (`suppress_backend_ime`), so only the activity's `EditText`
  decides when the keyboard is visible.
- The keyboard follows the focused widget, not one frame of its text-edit state.
  A number box (`DragValue` and a slider's value field) is a text edit only while
  it is being edited; it may drop out of that mode for a frame while its value
  changes. The bridge keeps the IME up across that frame and hides it only after
  two idle frames or when focus leaves, so typing a number does not make the
  keyboard flash.
- A typed number applies when editing finishes (Enter, focus loss, or a tap
  elsewhere), not on every digit. Partial values are not clamped or swapped
  under the keyboard.
- `BridgeInputConnection` forwards committed text and editing keys to
  `nativeImeText(String)` / `nativeImeKey(int)` in
  `crates/app/src/android_ime.rs`. The app turns them into ordinary
  `egui::Event`s, so the focused text field applies them exactly once.
- The `KEY_*` constants in `MainActivity.java` must match the `KEY_*` constants
  in `android_ime.rs`; a change on one side must change the other.
- Suggestions and autocorrect are disabled (`TYPE_TEXT_FLAG_NO_SUGGESTIONS`) so
  aliases, searches, and ABC source are not rewritten. Line breaks in committed
  text become Enter key presses.

This is separate from the media-session JNI path. Both use
`nativeAttachActivity` to pin the activity class, because the native event-loop
thread cannot look up app classes itself.

## File import and export

The saved-tune and saved-dance screens import and export local files. Rust calls
the static `openDocument(int, String, boolean)` and `shareText(String, String,
String)` methods on `MainActivity` through `crates/app/src/files.rs`; the
activity posts both to its UI thread.

- **Import** starts `ACTION_OPEN_DOCUMENT` for `text/vnd.abc` or
  `application/json`, accepts multiple ABC files, reads the chosen documents as
  UTF-8 with a 4 MB cap, and returns each one through
  `nativeImportedFile(int, String, String)`. The `int` is the import kind: 1 for
  ABC, 2 for dance JSON.
- **Export** writes the file into the app's private `files/config/exports`
  directory first, so saving never depends on the system picker. The UI then
  offers **Share**.
- **Share** copies the text into Downloads with `MediaStore` and sends an
  `ACTION_SEND` intent with the resulting `content://` URI and
  `FLAG_GRANT_READ_URI_PERMISSION`. A `FileProvider` would need a `<provider>`
  element in the manifest, which cargo-apk does not generate; `MediaStore`
  avoids both that manifest change and a `file://` URI. Android 8 and 9 have no
  Downloads collection, so those releases share `EXTRA_TEXT` only.

No manifest change is needed for this path: `ACTION_OPEN_DOCUMENT` and the
chooser are started with `startActivityForResult`/`startActivity`, and the
receiving app gets a read grant for the MediaStore URI.


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
