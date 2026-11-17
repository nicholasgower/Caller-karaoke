import {RELEASE_VERSION, releaseAssetUrl} from './release';

/**
 * What the download button can offer: one of the release builds, the browser
 * app hosted on this site (`hosted`), or iOS, which has no release build and
 * uses that hosted copy too.
 */
export type DownloadPlatform =
  | 'windows'
  | 'linux'
  | 'android'
  | 'web'
  | 'ios'
  | 'hosted';

export type DownloadAsset = {
  /** File name attached to the GitHub release; omitted for a link on this site. */
  file?: string;
  /** Button text when this asset is the main download. */
  action: string;
  /** Menu label for an alternative build. */
  label: string;
  /** One line of context under the menu label. */
  detail: string;
  /** Release asset URL, or a site path resolved against the deployed base URL. */
  url: string;
};

export type DownloadOptions = {
  /** Platform name used in accessible labels, e.g. "More downloads for Windows". */
  name: string;
  /** The build the main button downloads. */
  primary: DownloadAsset;
  /** Other builds for the same platform, offered in the menu. */
  alternatives: DownloadAsset[];
  /** Version note under the button; defaults to the unsigned release note. */
  caption?: string;
};

function asset(
  file: string,
  action: string,
  label: string,
  detail: string,
): DownloadAsset {
  return {file, action, label, detail, url: releaseAssetUrl(file)};
}

const WINDOWS_INSTALLER = `CallerKaraoke-${RELEASE_VERSION}-windows-x64-setup.exe`;
const WINDOWS_PORTABLE = `CallerKaraoke-${RELEASE_VERSION}-windows-x64-portable.zip`;
// cargo-deb appends the Debian revision, so the asset is `…_0.1.0-1_amd64.deb`.
const LINUX_DEB = `caller-app_${RELEASE_VERSION}-1_amd64.deb`;
const LINUX_APPIMAGE = `CallerKaraoke-${RELEASE_VERSION}-x86_64.AppImage`;
const ANDROID_APK = 'caller-karaoke.apk';
const WEB_ZIP = `caller-karaoke-${RELEASE_VERSION}-web.zip`;

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

/**
 * iOS has no release build yet, so the button opens the browser app hosted on
 * this site instead of downloading a Windows file that cannot run there. The
 * hosted copy is the same offer on any browser, where it needs no download.
 */
const HOSTED: DownloadOptions = {
  name: 'the hosted browser app',
  primary: {
    action: 'Play in your browser',
    label: 'Play in your browser',
    detail: 'Runs in this browser; no install needed.',
    url: '/play/',
  },
  alternatives: [],
  caption: `Version ${RELEASE_VERSION} · hosted browser build`,
};

const IOS: DownloadOptions = {
  name: 'iOS',
  primary: {
    ...HOSTED.primary,
    detail: 'Runs in Safari; no install needed.',
  },
  alternatives: [],
  caption: HOSTED.caption,
};

const OPTIONS: Record<DownloadPlatform, DownloadOptions> = {
  windows: WINDOWS,
  linux: LINUX,
  android: ANDROID,
  web: WEB,
  ios: IOS,
  hosted: HOSTED,
};

/** The builds offered for one platform; the first is the main button. */
export function downloadOptions(platform: DownloadPlatform): DownloadOptions {
  return OPTIONS[platform];
}

/**
 * The visitor's platform, read from the browser. iOS has no release build, so
 * it points at the browser app on this site; anything else without a build
 * (macOS, unknown systems) falls back to the Windows build rather than leaving
 * the visitor without a download.
 */
export function detectPlatform(): DownloadPlatform {
  if (typeof navigator === 'undefined') return 'windows';
  const reported = `${navigator.userAgent} ${navigator.platform ?? ''}`.toLowerCase();
  // iPadOS 13+ reports a desktop Safari user agent; multi-touch distinguishes it
  // from a Mac, which has no touch screen.
  const isIPadOS =
    reported.includes('macintosh') && navigator.maxTouchPoints > 1;
  if (
    reported.includes('iphone') ||
    reported.includes('ipad') ||
    reported.includes('ipod') ||
    isIPadOS
  ) {
    return 'ios';
  }
  if (reported.includes('android')) return 'android';
  if (reported.includes('windows') || reported.includes('win32') || reported.includes('win64')) {
    return 'windows';
  }
  if (reported.includes('linux') || reported.includes('x11')) return 'linux';
  return 'windows';
}
