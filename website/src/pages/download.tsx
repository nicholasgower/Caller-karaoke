import React from 'react';
import Layout from '@theme/Layout';
import Link from '@docusaurus/Link';

import DownloadButton from '@site/src/components/DownloadButton';
import {
  CHECKSUMS_URL,
  RELEASE_URL,
  RELEASE_VERSION,
} from '@site/src/release';

export default function Download(): React.ReactElement {
  return (
    <Layout
      title="Download"
      description="Download the latest Caller Karaoke release for Windows, Linux, web, or Android"
    >
      <main className="container margin-vert--lg">
        <h1>Download Caller Karaoke</h1>
        <p>
          Version <strong>{RELEASE_VERSION}</strong>. Choose a build below; each
          button downloads the release file directly. Builds are not code-signed
          yet, so Windows SmartScreen and some browsers may ask you to confirm
          the download.
        </p>

        <div className="row">
          <div className="col col--6 margin-bottom--lg">
            <h2>Windows</h2>
            <DownloadButton platform="windows" />
            <p>
              The installer sets up Caller Karaoke in Program Files; the portable
              ZIP runs from any folder without installing. Both need 64-bit
              Windows 10 or 11. See{' '}
              <Link to="/docs/getting-started/install-windows">Install on Windows</Link>.
            </p>
          </div>
          <div className="col col--6 margin-bottom--lg">
            <h2>Linux</h2>
            <DownloadButton platform="linux" />
            <p>
              The Debian package suits Debian, Ubuntu, and Mint; the AppImage runs
              on any 64-bit distribution. See{' '}
              <Link to="/docs/getting-started/install-linux">Install on Linux</Link>.
            </p>
          </div>
          <div className="col col--6 margin-bottom--lg">
            <h2>Browser (self-hosting)</h2>
            <DownloadButton platform="web" />
            <p>
              This bundle is for publishing your own copy of the browser build:
              serve it over HTTP, and add the included proxy for online search.
              To use the app without installing anything,{' '}
              <Link to="/play">play the hosted copy</Link> instead. See{' '}
              <Link to="/docs/getting-started/install-web">Run in the browser</Link>.
            </p>
          </div>
          <div className="col col--6 margin-bottom--lg">
            <h2>Android</h2>
            <DownloadButton platform="android" />
            <p>
              Sideload the APK on Android 8.0 or newer. It is a test build that
              uses the debug keystore, and real-device audio and TTS testing is
              still in progress. See{' '}
              <Link to="/docs/getting-started/install-android">Install on Android</Link>.
            </p>
          </div>
        </div>

        <h2>Verify the download</h2>
        <p>
          <a href={CHECKSUMS_URL}>SHA256SUMS.txt</a> lists a SHA-256 for every
          file in the release, and the{' '}
          <a href={RELEASE_URL}>release notes</a> record the commit and build
          time. Artifacts may be replaced by a later build of the same version.
        </p>

        <h2>Build from source</h2>
        <p>
          You need a recent stable Rust toolchain. The workspace declares Rust
          1.95 as its minimum.
        </p>
        <pre>
          <code>{`git clone https://github.com/nicholasgower/Caller-karaoke.git
cd Caller-karaoke
cargo run -p caller-app`}</code>
        </pre>
        <p>
          See <Link to="/docs/getting-started/build-and-run">Build and run</Link>{' '}
          for platform dependencies and the web/Android build commands.
        </p>

        <h2>Release caveats</h2>
        <p>
          Release artifacts carry the version shown on this page and may be
          replaced by a later build. Android CI uses the debug keystore unless
          release signing secrets are configured. Verify checksums before
          running an unsigned binary.
        </p>
      </main>
    </Layout>
  );
}
