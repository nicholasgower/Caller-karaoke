import React from 'react';

export default function Steps({children}: {children: React.ReactNode}): React.ReactElement {
  return <ol className="steps">{children}</ol>;
}
