---
title: Updates and feedback
description: Check GitHub for a newer Contra Karaoke release and send feedback from the Preferences header.
sidebar_position: 8
---

The **Preferences** header has two actions on the right, beside the page title:

- **Check for updates** asks GitHub for the newest published release and
  compares its version with the one this build is running.
- **Submit feedback** opens the project's GitHub issue tracker.

## Check for updates

Select **Check for updates** in the top-right corner of **Preferences**. The
result appears just below the title:

- **You are running the latest version** when the newest release is the same as
  or older than this build.
- **Version … is available** with an **Open the release page** link when GitHub
  has a newer release. Open the page to download the installer or package for
  your platform.
- **No GitHub release has been published yet** when the repository has no
  published release.
- **Could not check for updates** with the reason when the request fails or
  GitHub sends a response the app cannot read.

The version compared is the one shown in **Preferences → About**. Pre-release
tags are compared by their numeric version, so `v1.2.3-beta` is not newer than
`1.2.3`.

The check runs in the background and does not pause music or calls. Nothing is
downloaded or installed automatically, and the app does not send telemetry,
account details, or a token. The request is made only when you select the
button; the app does not check at startup.

The browser build asks the public GitHub API directly. GitHub serves that API
with cross-origin access enabled, so the web proxy is not involved. A browser, a
network firewall, or a GitHub outage can still make the check fail; the message
names the reason and you can try again later.

## Submit feedback

**Submit feedback** opens
[github.com/nicholasgower/Contra-karaoke/issues](https://github.com/nicholasgower/Contra-karaoke/issues)
in your browser. Reading issues does not need an account; posting one uses your
GitHub account. Include the app version from **Preferences → About** and, for a
bug, what you did and what happened.
