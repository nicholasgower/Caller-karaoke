import React, {useEffect, useId, useRef, useState} from 'react';
import useBaseUrl from '@docusaurus/useBaseUrl';

import {
  detectPlatform,
  downloadOptions,
  type DownloadPlatform,
} from '@site/src/downloads';
import {RELEASE_VERSION} from '@site/src/release';

import styles from './styles.module.css';

type Props = {
  /**
   * Which platform's builds to offer. Leave it off on pages that serve every
   * platform: the button then follows the visitor's operating system. `hosted`
   * offers the browser app running on this site, which needs no download.
   */
  platform?: DownloadPlatform | 'auto';
  /**
   * `button` (default) is the filled action a reader came to press. `link`
   * renders one inline link with the version note, for a build that is an
   * aside for technical readers rather than the page's main action.
   */
  variant?: 'button' | 'link';
};

export default function DownloadButton({
  platform = 'auto',
  variant = 'button',
}: Props): React.ReactElement {
  const menuId = useId();
  const rootRef = useRef<HTMLDivElement>(null);
  const toggleRef = useRef<HTMLButtonElement>(null);
  const [detected, setDetected] = useState<DownloadPlatform | null>(null);
  const [open, setOpen] = useState(false);

  // The static HTML ships the Windows build so the markup hydrates unchanged;
  // the visitor's own platform replaces it right after mount.
  const active = platform === 'auto' ? detected ?? 'windows' : platform;
  const options = downloadOptions(active);
  // The hosted copy and iOS point at the browser app, an in-site path that
  // needs the deployed base URL. Release URLs are absolute and pass through
  // unchanged.
  const primaryUrl = useBaseUrl(options.primary.url);

  useEffect(() => {
    if (platform === 'auto') setDetected(detectPlatform());
  }, [platform]);

  // A platform change while the menu is open would leave the wrong files there.
  useEffect(() => {
    setOpen(false);
  }, [active]);

  useEffect(() => {
    if (!open) return undefined;
    const closeOnOutsidePress = (event: PointerEvent) => {
      if (!rootRef.current?.contains(event.target as Node)) setOpen(false);
    };
    const closeOnEscape = (event: KeyboardEvent) => {
      if (event.key !== 'Escape') return;
      setOpen(false);
      toggleRef.current?.focus();
    };
    document.addEventListener('pointerdown', closeOnOutsidePress);
    document.addEventListener('keydown', closeOnEscape);
    return () => {
      document.removeEventListener('pointerdown', closeOnOutsidePress);
      document.removeEventListener('keydown', closeOnEscape);
    };
  }, [open]);

  // A link variant is one line, so it skips the menu and caption markup below.
  if (variant === 'link') {
    return (
      <p className={styles.link}>
        <a href={primaryUrl}>{options.primary.action}</a>
        <span className={styles.linkCaption}>
          {' · '}
          {options.caption ?? `Version ${RELEASE_VERSION} · unsigned build`}
        </span>
      </p>
    );
  }

  return (
    <div className={styles.download} ref={rootRef}>
      <div className={styles.buttons}>
        <a
          className={`button button--lg ${styles.main}`}
          href={primaryUrl}
          aria-describedby={`${menuId}-version`}
        >
          {options.primary.action}
        </a>
        {options.alternatives.length > 0 && (
          <>
            <button
              ref={toggleRef}
              type="button"
              className={`button button--lg ${styles.toggle}`}
              aria-expanded={open}
              aria-controls={menuId}
              aria-label={`More downloads for ${options.name}`}
              onClick={() => setOpen((value) => !value)}
            >
              <span className={styles.chevron} aria-hidden="true" />
            </button>
            {open && (
              <ul
                className={styles.menu}
                id={menuId}
                aria-label={`${options.name} downloads`}
              >
                {options.alternatives.map((asset) => (
                  <li key={asset.file}>
                    <a
                      className={styles.item}
                      href={asset.url}
                      onClick={() => setOpen(false)}
                    >
                      <span className={styles.itemLabel}>{asset.label}</span>
                      <span className={styles.itemDetail}>{asset.detail}</span>
                    </a>
                  </li>
                ))}
              </ul>
            )}
          </>
        )}
      </div>
      <p className={styles.caption} id={`${menuId}-version`}>
        {options.caption ?? `Version ${RELEASE_VERSION} · unsigned build`}
      </p>
    </div>
  );
}
