import React from 'react';
import Layout from '@theme/Layout';
import Link from '@docusaurus/Link';

export default function Download(): React.ReactElement {
  return (
    <Layout
      title="Download"
      description="Download the nightly Contra Karaoke build for Windows, Linux, web, or Android"
    >
      <main className="container margin-vert--lg">
        <h1>Download Contra Karaoke</h1>
        <p>
          Contra Karaoke is at <strong>0.1.0</strong> and is still in development.
          Nightly builds are produced on every push to <code>main</code> and
          attached to the rolling{' '}
          <a href="https://github.com/storytold/Contra-karaoke/releases/tag/nightly">
            nightly release
          </a>
          . There is no stable signed release yet.
        </p>

        <div className="row">
          <div className="col col--6 margin-bottom--lg">
            <h2>Windows</h2>
            <p>
              Download <code>ContraKaraoke-0.1.0-windows-x64-setup.exe</code> or{' '}
              <code>ContraKaraoke-0.1.0-windows-x64-portable.zip</code> from the
              nightly release. The installer is unsigned; see{' '}
              <Link to="/docs/getting-started/install-windows">Install on Windows</Link>{' '}
              before running it.
            </p>
          </div>
          <div className="col col--6 margin-bottom--lg">
            <h2>Linux</h2>
            <p>
              Download the <code>.deb</code> or{' '}
              <code>ContraKaraoke-0.1.0-x86_64.AppImage</code> from the nightly
              release. See{' '}
              <Link to="/docs/getting-started/install-linux">Install on Linux</Link>.
            </p>
          </div>
          <div className="col col--6 margin-bottom--lg">
            <h2>Web</h2>
            <p>
              Download <code>contra-karaoke-0.1.0-web.zip</code> and serve the
              bundle over HTTP. Online search needs the included proxy. See{' '}
              <Link to="/docs/getting-started/install-web">Run in the browser</Link>.
            </p>
          </div>
          <div className="col col--6 margin-bottom--lg">
            <h2>Android</h2>
            <p>
              Download the APK from the nightly release or build it with{' '}
              <code>cargo-apk</code>. Android still needs real-device testing. See{' '}
              <Link to="/docs/getting-started/install-android">Install on Android</Link>.
            </p>
          </div>
        </div>

        <h2>Build from source</h2>
        <p>
          You need a recent stable Rust toolchain. The workspace declares Rust
          1.95 as its minimum.
        </p>
        <pre>
          <code>{`git clone https://github.com/storytold/Contra-karaoke.git
cd Contra-karaoke
cargo run -p contra-app`}</code>
        </pre>
        <p>
          See <Link to="/docs/getting-started/build-and-run">Build and run</Link>{' '}
          for platform dependencies and the web/Android build commands.
        </p>

        <h2>Nightly caveats</h2>
        <p>
          Nightly artifacts use version 0.1.0, may be replaced by a later build,
          and are not code-signed. Android CI uses the debug keystore unless
          release signing secrets are configured. Verify checksums from the
          release before running an unsigned binary.
        </p>
      </main>
    </Layout>
  );
}
