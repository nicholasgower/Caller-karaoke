/**
 * Release metadata used by the download buttons.
 *
 * Release asset names carry the version, so `RELEASE_VERSION` mirrors
 * `[workspace.package] version` in `Cargo.toml`. `scripts/release.mjs` rewrites
 * this line in the commit that bumps the workspace version, which keeps every
 * button pointing at a file that exists.
 */
export const RELEASE_VERSION = '0.1.0';

export const REPO = 'nicholasgower/Contra-karaoke';

export const RELEASES_URL = `https://github.com/${REPO}/releases`;

export const RELEASE_TAG = `v${RELEASE_VERSION}`;

export const RELEASE_URL = `${RELEASES_URL}/tag/${RELEASE_TAG}`;

export const CHECKSUMS_URL = `${RELEASES_URL}/download/${RELEASE_TAG}/SHA256SUMS.txt`;

/** Direct download URL for one file attached to the current release. */
export function releaseAssetUrl(file: string): string {
  return `${RELEASES_URL}/download/${RELEASE_TAG}/${file}`;
}
