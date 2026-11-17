import {RELEASE_VERSION, releaseAssetUrl} from './release';

/** Release builds we publish, in the order the download buttons offer them. */
export type DownloadPlatform = 'windows' | 'linux' | 'android' | 'web';

export type DownloadAsset = {
  /** File name attached to the GitHub release. */
  file: string;
  /** Button text when this asset is the main download. */
  action: string;
  /** Menu label for an alternative build. */
  label: string;
  /** One line of context under the menu label. */
  detail: string;
  url: string;
};

export type DownloadOptions = {
  /** Platform name used in accessible labels, e.g. "More downloads for Windows". */
  name: string;
  /** The build the main button downloads. */
  primary: DownloadAsset;
  /** Other builds for the same platform, offered in the menu. */
  alternatives: DownloadAsset[];
};

function asset(
  file: string,
  action: string,
  label: string,
  detail: string,
): DownloadAsset {
  return {file, action, label, detail, url: releaseAssetUrl(file)};
}

const WINDOWS_INSTALLER = `ContraKaraoke-${RELEASE_VERSION}-windows-x64-setup.exe`;
const WINDOWS_PORTABLE = `ContraKaraoke-${RELEASE_VERSION}-windows-x64-portable.zip`;
// cargo-deb appends the Debian revision, so the asset is `…_0.1.0-1_amd64.deb`.
const LINUX_DEB = `contra-app_${RELEASE_VERSION}-1_amd64.deb`;
const LINUX_APPIMAGE = `ContraKaraoke-${RELEASE_VERSION}-x86_64.AppImage`;
const ANDROID_APK = 'contra-karaoke.apk';
const WEB_ZIP = `contra-karaoke-${RELEASE_VERSION}-web.zip`;

const WINDOWS: DownloadOptions = {
  name: 'Windows',
  primary: asset(
    WINDOWS_INSTALLER,
    'Download for Windows',
    'Windows installer',
    'Windows 10/11, 64-bit — recommended · .exe',
  ),
  alternatives: [
    asset(
      WINDOWS_PORTABLE,
      'Download for Windows',
      'Portable ZIP',
      'No installer; extract and run · .zip',
    ),
  ],
};

const LINUX: DownloadOptions = {
  name: 'Linux',
  primary: asset(
    LINUX_DEB,
    'Download for Linux',
    'Debian package',
    'Debian, Ubuntu, Mint — recommended · .deb',
  ),
  alternatives: [
    asset(
      LINUX_APPIMAGE,
      'Download for Linux',
      'AppImage',
      'Any distribution; make it executable · .AppImage',
    ),
  ],
};

const ANDROID: DownloadOptions = {
  name: 'Android',
  primary: asset(
    ANDROID_APK,
    'Download for Android',
    'Android APK',
    'Android 8.0 or newer · .apk',
  ),
  alternatives: [],
};

const WEB: DownloadOptions = {
  name: 'the browser build',
  primary: asset(
    WEB_ZIP,
    'Download the browser build',
    'Browser build',
    'WASM bundle; serve it over HTTP · .zip',
  ),
  alternatives: [],
};

const OPTIONS: Record<DownloadPlatform, DownloadOptions> = {
  windows: WINDOWS,
  linux: LINUX,
  android: ANDROID,
  web: WEB,
};

/** The builds offered for one platform; the first is the main button. */
export function downloadOptions(platform: DownloadPlatform): DownloadOptions {
  return OPTIONS[platform];
}

/**
 * The visitor's platform, read from the browser. Anything without a release
 * build (macOS, iOS, unknown systems) falls back to the Windows build rather
 * than leaving the visitor without a download.
 */
export function detectPlatform(): DownloadPlatform {
  if (typeof navigator === 'undefined') return 'windows';
  const reported = `${navigator.userAgent} ${navigator.platform ?? ''}`.toLowerCase();
  if (reported.includes('android')) return 'android';
  if (reported.includes('windows') || reported.includes('win32') || reported.includes('win64')) {
    return 'windows';
  }
  if (reported.includes('linux') || reported.includes('x11')) return 'linux';
  return 'windows';
}
