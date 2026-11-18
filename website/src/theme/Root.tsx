import React from 'react';
import useBaseUrl from '@docusaurus/useBaseUrl';

import {useWebAppPreload} from '@site/src/webAppPreload';

/**
 * Site-wide wrapper. Root sees every page, so the hosted browser app is warmed
 * from whichever link the visitor hovers: the navbar Play item, the landing-page
 * button, or a download button that points at `/play`.
 */
export default function Root({
  children,
}: {
  children: React.ReactNode;
}): React.ReactElement {
  useWebAppPreload(useBaseUrl('/app/'), useBaseUrl('/play/'));
  return <>{children}</>;
}
