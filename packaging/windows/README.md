# Windows packaging

The desktop build embeds ONNX Runtime 1.31.0 for the Kokoro voice. The build
script downloads the official MIT-licensed runtime once and embeds
`onnxruntime.dll` in the executable; the app writes it to the user cache on
first use. For an offline or audited build, point the script at a local copy:

```powershell
$env:CALLER_ONNX_RUNTIME_DLL = "C:\path\to\onnxruntime.dll"
cargo build --release -p caller-app
```

The voice model itself is downloaded from Hugging Face on first use and cached;
it is not part of the installer.

Build the release binary first:

```powershell
cargo build --release -p caller-app
```

Portable format: copy `target\release\caller-karaoke.exe` plus `LICENSE`,
`README.md`, `CREDITS.md`, and `CHANGELOG.md` into a folder and ZIP it.

Installer: install [Inno Setup](https://jrsoftware.org/isinfo.php) and run:

```powershell
iscc packaging\windows\caller-karaoke.iss `
  /DAppVersion=0.1.0 `
  /DOutputBaseFilename=CallerKaraoke-0.1.0-x64-setup
```

The installer is written to `dist\`. The release workflow passes the workspace
version from `Cargo.toml` as `AppVersion` and the output filename.

## Media keys

The binary registers System Media Transport Controls when it starts, using its
own window handle. There is nothing to install or register: the keyboard's
**Play/Pause** key, the media overlay, and the Quick Settings media controls
find the app while it is running, and the overlay shows the loaded dance and
tune. The portable ZIP and the installer behave the same way, since neither
needs a registry entry for this.
