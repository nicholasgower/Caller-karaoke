---
title: Install on Windows
description: Run the Windows installer or portable ZIP, or build Contra Karaoke from source.
sidebar_position: 1
---

Contra Karaoke is distributed as a 64-bit Windows installer and a portable ZIP
in the rolling nightly release. There is no stable signed release yet, so
Windows may show a SmartScreen warning.

## Requirements

- 64-bit Windows 10 or 11.
- An audio output device. Windows TTS voices are optional.
- Network access for the first dance or tune search. The bundled defaults work
  offline.

## Download

Open the [nightly release](https://github.com/storytold/Contra-karaoke/releases/tag/nightly)
and download one of:

- `ContraKaraoke-0.1.0-windows-x64-setup.exe` — installer;
- `ContraKaraoke-0.1.0-windows-x64-portable.zip` — portable build.

Nightly artifacts carry version 0.1.0 and may be replaced by a later build.
They are not code-signed.

<Shot id="install-windows-release" />

## Install with the setup program

<Steps>
  <Step shot="install-windows-wizard">
    Run the setup program and follow the Inno Setup wizard. Choose whether to
    create a desktop shortcut, then finish the install.
  </Step>
  <Step shot="install-windows-smartscreen">
    If Windows SmartScreen appears because the nightly build is unsigned,
    select **More info**, verify the publisher/URL, and choose **Run anyway**
    only if you downloaded it from this repository's release page.
  </Step>
</Steps>

The installer writes to the standard Program Files location and registers an
uninstaller. It does not install a service.

## Use the portable ZIP

<Steps>
  <Step shot="install-windows-portable">
    Extract the ZIP to a folder you can write to, then run
    `contra-karaoke.exe`.
  </Step>
</Steps>

The portable build does not write to the registry. Preferences and the source
cache are still stored in the standard per-user configuration and cache
directories, so the app behaves the same across installer and portable builds.

## First launch

The app opens on **Session** with *Another Last Dance* and *Sandy Boys*. Press
**Play** to build the music; the first call may start during the count-in. See
[Your first session](/docs/getting-started/first-session).

If SmartScreen or antivirus blocks the unsigned executable, verify the download
hash from the nightly release and use the **More info** path rather than
disabling security features.

## Uninstall

Use **Settings → Apps → Installed apps → Contra Karaoke → Uninstall**. For the
portable build, delete the extracted folder. Removing the app does not
automatically remove the per-user cache or preferences.

## Build from source instead

If you prefer not to run an unsigned binary, build the desktop app from source.
See [Build and run](/docs/getting-started/build-and-run) for the Rust toolchain and commands.

Next: [Your first session](/docs/getting-started/first-session).
