import React from 'react';
import Layout from '@theme/Layout';
import Link from '@docusaurus/Link';
import useDocusaurusContext from '@docusaurus/useDocusaurusContext';

import DownloadButton from '@site/src/components/DownloadButton';

import styles from './index.module.css';

export default function Home(): React.ReactElement {
  const {siteConfig} = useDocusaurusContext();
  return (
    <Layout title="Contra Karaoke" description={siteConfig.tagline}>
      <header className={styles.hero}>
        <div className="container">
          <p className={styles.kicker}>Caller training for contra dances</p>
          <h1 className={styles.title}>Contra Karaoke</h1>
          <p className={styles.subtitle}>{siteConfig.tagline}</p>
          <p className={styles.lede}>
            The dance card and the music move together. Each call appears and is
            spoken early enough that the words finish before the dancers move.
          </p>
          <div className={styles.download}>
            <DownloadButton />
          </div>
          <div className={styles.actions}>
            <Link className="button button--primary button--lg" to="/play">
              Play in your browser
            </Link>
            <Link
              className="button button--primary button--lg"
              to="/docs/getting-started/first-session"
            >
              Run your first session
            </Link>
            <Link
              className="button button--primary button--lg"
              to="/docs/intro"
            >
              Read the guide
            </Link>
          </div>
          <p className={styles.note}>
            0.1.0 release builds for Windows, Linux, web, and Android. A hosted
            browser copy is available on this site. Android audio and TTS are
            still being device-tested.
          </p>
        </div>
      </header>
      <main className="container">
        <section className={styles.grid}>
          <article className={styles.card}>
            <h2>Choose a real dance</h2>
            <p>
              Search ContraDB by title, choreographer, formation, figure words,
              or verified status, then preview the full card before loading it.
            </p>
            <Link to="/docs/guides/choosing-a-dance">Choosing a dance →</Link>
          </article>
          <article className={styles.card}>
            <h2>Bring your own tune</h2>
            <p>
              Search ABC Session Tunes by title, composer, key, meter, or genre.
              The arranged loop is always 32 bars, with a four-beat count-in.
            </p>
            <Link to="/docs/guides/choosing-a-tune">Choosing a tune →</Link>
          </article>
          <article className={styles.card}>
            <h2>Make the calls yours</h2>
            <p>
              Change role terms, edit move aliases, pick instruments, and adjust
              how early calls begin. Longer calls are moved earlier automatically.
            </p>
            <Link to="/docs/guides/aliases">Editing aliases →</Link>
          </article>
        </section>
      </main>
    </Layout>
  );
}
