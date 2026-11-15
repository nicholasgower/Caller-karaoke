import React from 'react';
import Layout from '@theme/Layout';
import Link from '@docusaurus/Link';
import useBaseUrl from '@docusaurus/useBaseUrl';

import styles from './play.module.css';

export default function Play(): React.ReactElement {
  const appUrl = useBaseUrl('/app/');

  return (
    <Layout
      title="Play in your browser"
      description="Try Contra Karaoke without installing it. The hosted browser build plays the bundled dance and tune; online search needs a data proxy."
      noFooter
    >
      <main className={styles.page}>
        <div className={styles.bar}>
          <p className={styles.note}>
            This copy is built from <code>main</code> with the documentation and
            may lag the nightly release. Press <strong>Play</strong> in the app
            once to allow audio. Online search needs a{' '}
            <Link to="/docs/getting-started/install-web#configure-the-data-proxy">
              data proxy
            </Link>
            .
          </p>
          <a
            className={styles.open}
            href={appUrl}
            target="_blank"
            rel="noopener noreferrer"
          >
            Open full screen
          </a>
        </div>
        <iframe
          className={styles.frame}
          src={appUrl}
          title="Contra Karaoke browser app"
          allow="autoplay; fullscreen"
        />
      </main>
    </Layout>
  );
}
