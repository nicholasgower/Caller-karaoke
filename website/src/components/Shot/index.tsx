import React from 'react';
import useBaseUrl from '@docusaurus/useBaseUrl';
import manifest from '../../../screenshots/manifest.json';

type ShotRecord = {
  id: string;
  alt: string;
  caption?: string;
  status?: 'pending' | 'ready';
  width?: number;
  height?: number;
  priority?: boolean;
};

type Props = {
  id: string;
  caption?: string;
};

export default function Shot({id, caption}: Props): React.ReactElement {
  const shot = (manifest.shots as ShotRecord[]).find((item) => item.id === id);
  if (!shot) {
    throw new Error(`Unknown screenshot "${id}". Add it to website/screenshots/manifest.json.`);
  }

  const src = useBaseUrl(`/img/screens/${id}.png`);
  const captionText = caption ?? shot.caption;

  return (
    <figure className="shot">
      {shot.status === 'ready' ? (
        <a href={src} target="_blank" rel="noreferrer">
          <img
            src={src}
            alt={shot.alt}
            width={shot.width}
            height={shot.height}
            loading={shot.priority ? 'eager' : 'lazy'}
            decoding="async"
          />
        </a>
      ) : (
        <div className="shot__pending" role="img" aria-label={shot.alt}>
          <span>Screenshot pending</span>
          <code>{id}</code>
        </div>
      )}
      {captionText ? <figcaption>{captionText}</figcaption> : null}
    </figure>
  );
}
