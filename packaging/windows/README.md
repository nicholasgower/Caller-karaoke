# Windows packaging

Build the release binary first:

```powershell
cargo build --release -p contra-app
```

Portable format: copy `target\release\contra-karaoke.exe` plus `LICENSE`,
`README.md`, `CREDITS.md`, and `CHANGELOG.md` into a folder and ZIP it.

Installer: install [Inno Setup](https://jrsoftware.org/isinfo.php) and run:

```powershell
iscc packaging\windows\contra-karaoke.iss `
  /DAppVersion=0.1.0 `
  /DOutputBaseFilename=ContraKaraoke-0.1.0-x64-setup
```

The installer is written to `dist\`. Nightly CI overrides the version and
filename with the current date and short SHA.
